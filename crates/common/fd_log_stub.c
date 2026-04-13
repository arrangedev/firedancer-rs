#define _GNU_SOURCE
#include <stdio.h>
#include <stdarg.h>
#include <stdlib.h>
#include <time.h>

char const *
fd_log_private_0( char const * fmt, ... ) {
  static _Thread_local char buf[1024];
  va_list ap;
  va_start(ap, fmt);
  vsnprintf(buf, sizeof(buf), fmt, ap);
  va_end(ap);
  return buf;
}

void
fd_log_private_1( int          level,
                  long         now,
                  char const * file,
                  int          line,
                  char const * func,
                  char const * msg ) {
  (void)now;
  if( level >= 3 ) {
    fprintf(stderr, "[%s:%d] %s: %s\n", file, line, func, msg);
  }
}

void
fd_log_private_2( int          level,
                  long         now,
                  char const * file,
                  int          line,
                  char const * func,
                  char const * msg ) {
  (void)level;
  (void)now;
  fprintf(stderr, "FATAL [%s:%d] %s: %s\n", file, line, func, msg);
  abort();
}

long
fd_log_wallclock( void ) {
  struct timespec ts;
  clock_gettime(CLOCK_REALTIME, &ts);
  return ((long)1e9) * ((long)ts.tv_sec) + (long)ts.tv_nsec;
}
