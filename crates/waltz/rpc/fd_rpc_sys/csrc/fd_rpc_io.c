#include "fd_rpc_io.h"
#include "h2/fd_h2_rbuf_sock.h"

#if FD_HAS_OPENSSL
#include "h2/fd_h2_rbuf_ossl.h"
#endif

#include <errno.h>
#include <unistd.h>
#include <fcntl.h>
#include <poll.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <string.h>

#ifdef __APPLE__

/* apple silicon -- macOS has no SOCK_NONBLOCK or MSG_NOSIGNAL */

#ifndef MSG_NOSIGNAL
#define MSG_NOSIGNAL 0
#endif

static int
fd_rpc_io_socket_nb( int domain, int type, int protocol ) {
  int fd = socket( domain, type, protocol );
  if( fd<0 ) return fd;
  int flags = fcntl( fd, F_GETFL, 0 );
  if( flags<0 || fcntl( fd, F_SETFL, flags|O_NONBLOCK )<0 ) {
    close( fd );
    return -1;
  }
  int on = 1;
  setsockopt( fd, SOL_SOCKET, SO_NOSIGPIPE, &on, sizeof(on) );
  return fd;
}

#else /* Linux */

static int
fd_rpc_io_socket_nb( int domain, int type, int protocol ) {
  return socket( domain, type|SOCK_NONBLOCK, protocol );
}

#endif /* __APPLE__ */

fd_rpc_io_t *
fd_rpc_io_new( void * mem,
               void * rx_buf,
               ulong  rx_bufsz,
               void * tx_buf,
               ulong  tx_bufsz ) {
  if( FD_UNLIKELY( !mem ) ) return NULL;

  fd_rpc_io_t * io = (fd_rpc_io_t *)mem;
  memset( io, 0, sizeof(fd_rpc_io_t) );
  io->sock_fd = -1;
  io->state   = FD_RPC_IO_STATE_DISCONNECTED;
  fd_h2_rbuf_init( io->rbuf_rx, rx_buf, rx_bufsz );
  fd_h2_rbuf_init( io->rbuf_tx, tx_buf, tx_bufsz );
  return io;
}

int
fd_rpc_io_connect( fd_rpc_io_t * io,
                   uint          addr,
                   ushort        port,
                   int           use_tls,
                   char const *  hostname ) {
  if( FD_UNLIKELY( io->state != FD_RPC_IO_STATE_DISCONNECTED ) ) {
    return -1;
  }

  int fd = fd_rpc_io_socket_nb( AF_INET, SOCK_STREAM, 0 );
  if( FD_UNLIKELY( fd<0 ) ) {
    io->err   = errno;
    io->state = FD_RPC_IO_STATE_ERROR;
    return -1;
  }

  int opt = 1;
  setsockopt( fd, IPPROTO_TCP, TCP_NODELAY, &opt, sizeof(opt) );

  struct sockaddr_in sa = {
    .sin_family = AF_INET,
    .sin_port   = fd_ushort_bswap( port ),
    .sin_addr   = { .s_addr = addr }
  };

  int rc = connect( fd, (struct sockaddr *)&sa, sizeof(sa) );
  if( FD_UNLIKELY( rc<0 && errno!=EINPROGRESS ) ) {
    io->err = errno;
    close( fd );
    io->state = FD_RPC_IO_STATE_ERROR;
    return -1;
  }

  io->sock_fd = fd;

#if FD_HAS_OPENSSL
  if( use_tls ) {
    io->ssl_ctx = SSL_CTX_new( TLS_client_method() );
    if( FD_UNLIKELY( !io->ssl_ctx ) ) {
      close( fd );
      io->sock_fd = -1;
      io->state   = FD_RPC_IO_STATE_ERROR;
      return -1;
    }
    SSL_CTX_set_default_verify_paths( io->ssl_ctx );
    SSL_CTX_set_verify( io->ssl_ctx, SSL_VERIFY_PEER, NULL );

    io->ssl = SSL_new( io->ssl_ctx );
    if( FD_UNLIKELY( !io->ssl ) ) {
      SSL_CTX_free( io->ssl_ctx );
      io->ssl_ctx = NULL;
      close( fd );
      io->sock_fd = -1;
      io->state   = FD_RPC_IO_STATE_ERROR;
      return -1;
    }

    SSL_set_fd( io->ssl, fd );
    SSL_set_connect_state( io->ssl );

    if( hostname ) {
      SSL_set_tlsext_host_name( io->ssl, hostname );
      SSL_set1_host( io->ssl, hostname );
    }

    io->ssl_hs_done = 0;
    io->state = ( rc==0 ) ? FD_RPC_IO_STATE_TLS_HANDSHAKE
                          : FD_RPC_IO_STATE_CONNECTING;
  } else
#else
  (void)use_tls;
  (void)hostname;
#endif
  {
    io->state = ( rc==0 ) ? FD_RPC_IO_STATE_READY
                          : FD_RPC_IO_STATE_CONNECTING;
  }

  return 0;
}

static uint
fd_rpc_io_pump_connecting( fd_rpc_io_t * io ) {
  struct pollfd pfd = {
    .fd      = io->sock_fd,
    .events  = POLLOUT,
    .revents = 0
  };
  int nfds = poll( &pfd, 1, 0 );
  if( nfds<=0 ) return 0;

  if( pfd.revents & (POLLERR|POLLHUP) ) {
    int so_err = 0;
    socklen_t len = sizeof(so_err);
    getsockopt( io->sock_fd, SOL_SOCKET, SO_ERROR, &so_err, &len );
    io->err   = so_err;
    io->state = FD_RPC_IO_STATE_ERROR;
    return FD_RPC_IO_PUMP_ERROR;
  }

  if( pfd.revents & POLLOUT ) {
#if FD_HAS_OPENSSL
    if( io->ssl ) {
      io->state = FD_RPC_IO_STATE_TLS_HANDSHAKE;
      return FD_RPC_IO_PUMP_CONNECTED;
    }
#endif
    io->state = FD_RPC_IO_STATE_READY;
    return FD_RPC_IO_PUMP_CONNECTED;
  }
  return 0;
}

#if FD_HAS_OPENSSL
static uint
fd_rpc_io_pump_tls( fd_rpc_io_t * io ) {
  if( io->ssl_hs_done ) return 0;
  int res = SSL_do_handshake( io->ssl );
  if( res<=0 ) {
    int error = SSL_get_error( io->ssl, res );
    if( error==SSL_ERROR_WANT_READ || error==SSL_ERROR_WANT_WRITE ) return 0;
    io->err   = error;
    io->state = FD_RPC_IO_STATE_ERROR;
    return FD_RPC_IO_PUMP_ERROR;
  }
  io->ssl_hs_done = 1;
  io->state       = FD_RPC_IO_STATE_READY;
  return FD_RPC_IO_PUMP_CONNECTED;
}
#endif

static uint
fd_rpc_io_pump_ready( fd_rpc_io_t * io ) {
  uint flags = 0U;

#if FD_HAS_OPENSSL
  if( io->ssl ) {
    int ssl_err = 0;
    ulong read_sz = fd_h2_rbuf_ssl_read( io->rbuf_rx, io->ssl, &ssl_err );
    if( FD_UNLIKELY( ssl_err && ssl_err!=SSL_ERROR_WANT_READ ) ) {
      io->err   = ssl_err;
      io->state = FD_RPC_IO_STATE_ERROR;
      return FD_RPC_IO_PUMP_ERROR | FD_RPC_IO_PUMP_CLOSED;
    }
    if( read_sz ) flags |= FD_RPC_IO_PUMP_RX_DATA;

    ulong write_sz = fd_h2_rbuf_ssl_write( io->rbuf_tx, io->ssl );
    if( write_sz ) flags |= FD_RPC_IO_PUMP_TX_DRAIN;

    return flags;
  }
#endif

  int rx_err = fd_h2_rbuf_recvmsg( io->rbuf_rx, io->sock_fd, MSG_NOSIGNAL|MSG_DONTWAIT );
  if( FD_UNLIKELY( rx_err ) ) {
    if( rx_err==EPIPE ) {
      io->state = FD_RPC_IO_STATE_ERROR;
      return FD_RPC_IO_PUMP_CLOSED;
    }
    io->err   = rx_err;
    io->state = FD_RPC_IO_STATE_ERROR;
    return FD_RPC_IO_PUMP_ERROR;
  }
  if( fd_h2_rbuf_used_sz( io->rbuf_rx ) ) flags |= FD_RPC_IO_PUMP_RX_DATA;

  int tx_err = fd_h2_rbuf_sendmsg( io->rbuf_tx, io->sock_fd, MSG_NOSIGNAL|MSG_DONTWAIT );
  if( FD_UNLIKELY( tx_err ) ) {
    io->err   = tx_err;
    io->state = FD_RPC_IO_STATE_ERROR;
    return FD_RPC_IO_PUMP_ERROR;
  }
  if( fd_h2_rbuf_free_sz( io->rbuf_tx ) ) flags |= FD_RPC_IO_PUMP_TX_DRAIN;

  return flags;
}

uint
fd_rpc_io_pump( fd_rpc_io_t * io ) {
  switch( io->state ) {
  case FD_RPC_IO_STATE_CONNECTING:
    return fd_rpc_io_pump_connecting( io );
  case FD_RPC_IO_STATE_READY:
    return fd_rpc_io_pump_ready( io );
#if FD_HAS_OPENSSL
  case FD_RPC_IO_STATE_TLS_HANDSHAKE:
    return fd_rpc_io_pump_tls( io );
#endif
  default:
    return 0;
  }
}

void
fd_rpc_io_close( fd_rpc_io_t * io ) {
#if FD_HAS_OPENSSL
  if( io->ssl ) {
    SSL_shutdown( io->ssl );
    SSL_free( io->ssl );
    io->ssl = NULL;
  }
  if( io->ssl_ctx ) {
    SSL_CTX_free( io->ssl_ctx );
    io->ssl_ctx = NULL;
  }
#endif
  if( io->sock_fd >= 0 ) {
    close( io->sock_fd );
    io->sock_fd = -1;
  }
  io->state = FD_RPC_IO_STATE_DISCONNECTED;
}
