/* Stub linux/mempolicy.h for macOS */
#ifndef _LINUX_MEMPOLICY_H
#define _LINUX_MEMPOLICY_H

/* Memory policy modes */
#define MPOL_DEFAULT     0
#define MPOL_PREFERRED   1  
#define MPOL_BIND        2
#define MPOL_INTERLEAVE  3
#define MPOL_LOCAL       4

/* Memory policy flags */
#define MPOL_F_NODE      (1<<0)
#define MPOL_F_ADDR      (1<<1)
#define MPOL_F_MEMS_ALLOWED (1<<2)

/* Memory policy flags */
#define MPOL_F_STATIC_NODES (1<<15)
#define MPOL_F_RELATIVE_NODES (1<<14)

/* mbind flags */
#define MPOL_MF_STRICT   (1<<0)
#define MPOL_MF_MOVE     (1<<1)
#define MPOL_MF_MOVE_ALL (1<<2)

/* fallocate flags */
#define FALLOC_FL_KEEP_SIZE 0x01

/* Stub functions */
static inline int fallocate(int fd, int mode, off_t offset, off_t len) {
    (void)fd; (void)mode; (void)offset; (void)len;
    return 0; /* Always succeed on macOS */
}

static inline int set_mempolicy(int mode, const unsigned long *nodemask, unsigned long maxnode) {
    (void)mode; (void)nodemask; (void)maxnode;
    return 0; /* Always succeed on macOS */
}

static inline int get_mempolicy(int *mode, unsigned long *nodemask, unsigned long maxnode, 
                               void *addr, unsigned long flags) {
    (void)mode; (void)nodemask; (void)maxnode; (void)addr; (void)flags;
    return 0; /* Always succeed on macOS */
}

static inline long mbind(void *addr, unsigned long len, int mode, const unsigned long *nodemask,
                        unsigned long maxnode, unsigned flags) {
    (void)addr; (void)len; (void)mode; (void)nodemask; (void)maxnode; (void)flags;
    return 0; /* Always succeed on macOS */
}

#endif /* _LINUX_MEMPOLICY_H */
