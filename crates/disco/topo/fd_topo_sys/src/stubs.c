#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>

void fd_log_private_0(const char* msg) {
    fprintf(stderr, "[LOG] %s\n", msg);
}

void fd_log_private_1(const char* fmt, const char* arg) {
    fprintf(stderr, "[LOG] ");
    fprintf(stderr, fmt, arg);
    fprintf(stderr, "\n");
}

void fd_log_private_2(const char* fmt, const char* arg1, const char* arg2) {
    fprintf(stderr, "[LOG] ");
    fprintf(stderr, fmt, arg1, arg2);
    fprintf(stderr, "\n");
}

void fd_log_private_fprintf_nolock_0(FILE* stream, const char* msg) {
    fprintf(stream, "%s", msg);
}

long fd_log_wallclock(void) {
    return 0;
}

int fd_cstr_printf_check(char* buf, size_t buf_sz, const char* fmt, ...) {
    (void)buf; (void)buf_sz; (void)fmt;
    return 0;
}

unsigned long fd_cstr_to_ulong(const char* str) {
    return strtoul(str, NULL, 10);
}

const char* fd_io_strerror(int errnum) {
    return strerror(errnum);
}

unsigned long fd_numa_node_cnt(void) {
    return 1;
}

unsigned long fd_numa_node_idx(void) {
    return 0;
}

unsigned long fd_shmem_numa_cnt(void) {
    return 1;
}

unsigned long fd_shmem_numa_idx(void) {
    return 0;
}

const char* fd_shmem_page_sz_to_cstr(unsigned long page_sz) {
    static char buf[32];
    snprintf(buf, sizeof(buf), "%lu", page_sz);
    return buf;
}

void fd_wksp_detach(void* wksp) {
    (void)wksp; 
}

unsigned long fd_wksp_footprint(void* wksp) {
    (void)wksp;
    return 0;
}

void* fd_pod_alloc(void* pod, unsigned long sz) {
    (void)pod;
    return malloc(sz);
}

void* fd_pod_query(void* pod, const char* key, void* default_val) {
    (void)pod; (void)key;
    return default_val;
}

void fd_pod_remove(void* pod, const char* key) {
    (void)pod; (void)key;
}

unsigned long fd_dcache_req_data_sz(unsigned long depth, unsigned long data_sz, unsigned long burst) {
    return depth * data_sz * burst;
}

unsigned long fd_topo_find_wksp__extern(void* topo, const char* name) {
    (void)topo; (void)name;
    return (unsigned long)-1;
}

unsigned long fd_topo_find_tile__extern(void* topo, const char* name, unsigned long kind_id) {
    (void)topo; (void)name; (void)kind_id;
    return (unsigned long)-1;
}

unsigned long fd_topo_find_link__extern(void* topo, const char* name, unsigned long kind_id) {
    (void)topo; (void)name; (void)kind_id;
    return (unsigned long)-1;
}

unsigned long fd_topo_tile_name_cnt__extern(void* topo, const char* name) {
    (void)topo; (void)name;
    return 0;
}
