/* Stub linux/mman.h for macOS */
#ifndef _LINUX_MMAN_H
#define _LINUX_MMAN_H

/* Include the standard mman.h first */
#include <sys/mman.h>

/* Linux-specific memory mapping flags */
#ifndef MAP_HUGETLB
#define MAP_HUGETLB      0x40000
#endif

#ifndef MAP_HUGE_SHIFT
#define MAP_HUGE_SHIFT   26
#endif

#ifndef MAP_HUGE_MASK
#define MAP_HUGE_MASK    0x3f
#endif

#ifndef MAP_HUGE_2MB
#define MAP_HUGE_2MB    (21 << MAP_HUGE_SHIFT)
#endif

#ifndef MAP_HUGE_1GB  
#define MAP_HUGE_1GB    (30 << MAP_HUGE_SHIFT)
#endif

#endif /* _LINUX_MMAN_H */
