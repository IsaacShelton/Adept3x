
#ifndef _C_ERRNO_H_INCLUDED
#define _C_ERRNO_H_INCLUDED

thread_local extern int errno;

char *strerror(int errnum);

#endif // _C_ERRNO_H_INCLUDED

