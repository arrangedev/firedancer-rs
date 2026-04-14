#include "ed25519/fd_curve25519.h"

int
fd_ed25519_point_is_on_curve( uchar const buf[ 32 ] ) {
  fd_f25519_t x[1], y[1];
  fd_f25519_frombytes( y, buf );

  fd_f25519_t u[1];
  fd_f25519_t v[1];
  fd_f25519_sqr( u, y                );
  fd_f25519_mul( v, u, fd_f25519_d   );
  fd_f25519_sub( u, u, fd_f25519_one );
  fd_f25519_add( v, v, fd_f25519_one );

  return fd_f25519_sqrt_ratio( x, u, v );
}
