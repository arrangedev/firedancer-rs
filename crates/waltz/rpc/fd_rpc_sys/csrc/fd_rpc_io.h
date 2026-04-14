#ifndef HEADER_fd_rpc_io_h
#define HEADER_fd_rpc_io_h

/* fd_rpc_io.h provides a connection handle for non-blocking TCP I/O
   with optional TLS.  Wraps a socket fd, optional SSL handle, and
   dual ring buffers (RX/TX) backed by caller-provided memory.

   The caller is responsible for framing (HTTP/1.1,
   JSON-RPC, etc...) on top of this transport. See the Rust impl for more
   on this. */

#include "h2/fd_h2_rbuf.h"

#if FD_HAS_OPENSSL
#include <openssl/ssl.h>
#include <openssl/err.h>
#endif

#define FD_RPC_IO_STATE_DISCONNECTED  (0)
#define FD_RPC_IO_STATE_CONNECTING    (1)
#define FD_RPC_IO_STATE_TLS_HANDSHAKE (2)
#define FD_RPC_IO_STATE_READY         (3)
#define FD_RPC_IO_STATE_ERROR         (4)

#define FD_RPC_IO_PUMP_CONNECTED (1U<<0)
#define FD_RPC_IO_PUMP_RX_DATA   (1U<<1)
#define FD_RPC_IO_PUMP_TX_DRAIN  (1U<<2)
#define FD_RPC_IO_PUMP_ERROR     (1U<<3)
#define FD_RPC_IO_PUMP_CLOSED    (1U<<4)

#define FD_RPC_IO_ALIGN     (8UL)

struct fd_rpc_io {
  int           sock_fd;
  int           state;
  int           err;

#if FD_HAS_OPENSSL
  SSL_CTX *     ssl_ctx;
  SSL *         ssl;
  int           ssl_hs_done;
#endif

  fd_h2_rbuf_t  rbuf_rx[1];
  fd_h2_rbuf_t  rbuf_tx[1];
};

typedef struct fd_rpc_io fd_rpc_io_t;

FD_PROTOTYPES_BEGIN

FD_FN_CONST static inline ulong
fd_rpc_io_align( void ) {
  return FD_RPC_IO_ALIGN;
}

FD_FN_CONST static inline ulong
fd_rpc_io_footprint( void ) {
  return sizeof(fd_rpc_io_t);
}

fd_rpc_io_t *
fd_rpc_io_new( void * mem,
               void * rx_buf,
               ulong  rx_bufsz,
               void * tx_buf,
               ulong  tx_bufsz );

int
fd_rpc_io_connect( fd_rpc_io_t * io,
                   uint          addr,
                   ushort        port,
                   int           use_tls,
                   char const *  hostname );

uint
fd_rpc_io_pump( fd_rpc_io_t * io );

void
fd_rpc_io_close( fd_rpc_io_t * io );

static inline fd_h2_rbuf_t *
fd_rpc_io_rbuf_rx( fd_rpc_io_t * io ) {
  return io->rbuf_rx;
}

static inline fd_h2_rbuf_t *
fd_rpc_io_rbuf_tx( fd_rpc_io_t * io ) {
  return io->rbuf_tx;
}

static inline int
fd_rpc_io_state( fd_rpc_io_t const * io ) {
  return io->state;
}

FD_PROTOTYPES_END

#endif /* HEADER_fd_rpc_io_h */
