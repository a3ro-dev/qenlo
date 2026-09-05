#ifndef QENLO_H
#define QENLO_H
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
typedef struct QenloCollection QenloCollection;
typedef struct QenloSnapshot QenloSnapshot;
typedef struct QenloSearchResults QenloSearchResults;
enum {
    QENLO_BACKEND_CPU = 0,
    QENLO_BACKEND_AUTOMATIC = 1,
    QENLO_BACKEND_GPU_REQUIRED = 2,
};
enum {
    QENLO_GPU_FILTER_CPU_MASK = 0,
    QENLO_GPU_FILTER_CPU_ROWS = 1,
    QENLO_GPU_FILTER_PREDICATE = 2,
};
QenloCollection *qenlo_collection_new(size_t dimension);
QenloCollection *qenlo_collection_new_configured(size_t dimension, uint32_t backend, uint32_t gpu_filter_mode, uint64_t gpu_allocation_budget_bytes);
QenloCollection *qenlo_collection_create(const char *path, size_t dimension);
QenloCollection *qenlo_collection_create_configured(const char *path, size_t dimension, uint32_t backend, uint32_t gpu_filter_mode, uint64_t gpu_allocation_budget_bytes);
QenloCollection *qenlo_collection_open(const char *path, size_t dimension);
QenloCollection *qenlo_collection_open_configured(const char *path, size_t dimension, uint32_t backend, uint32_t gpu_filter_mode, uint64_t gpu_allocation_budget_bytes);
QenloCollection *qenlo_collection_import_qn(const char *path, size_t dimension);
QenloCollection *qenlo_collection_import_qn_configured(const char *path, size_t dimension, uint32_t backend, uint32_t gpu_filter_mode, uint64_t gpu_allocation_budget_bytes);
int32_t qenlo_add(QenloCollection *, uint64_t, uint64_t, int64_t, const float *, size_t);
int32_t qenlo_add_batch(QenloCollection *, const uint64_t *, const uint64_t *, const int64_t *, const float *, size_t, size_t);
int32_t qenlo_delete(QenloCollection *, uint64_t);
int32_t qenlo_delete_batch(QenloCollection *, const uint64_t *, size_t);
char *qenlo_search(QenloCollection *, const float *, size_t, bool, uint64_t, bool, int64_t, bool, int64_t, size_t);
QenloSearchResults *qenlo_search_results_new(QenloCollection *, const float *, size_t, bool, uint64_t, bool, int64_t, bool, int64_t, size_t);
int32_t qenlo_search_results_len(QenloSearchResults *, size_t *);
int32_t qenlo_search_results_copy(QenloSearchResults *, uint64_t *, size_t, float *, size_t);
char *qenlo_search_results_report_json(QenloSearchResults *);
void qenlo_search_results_free(QenloSearchResults *);
QenloSnapshot *qenlo_snapshot_new(QenloCollection *, bool, uint64_t, bool, int64_t, bool, int64_t);
int32_t qenlo_snapshot_info(QenloSnapshot *, uint64_t *, size_t *, size_t *);
int32_t qenlo_snapshot_copy(QenloSnapshot *, uint64_t *, size_t, float *, size_t);
void qenlo_snapshot_free(QenloSnapshot *);
int32_t qenlo_collection_generation(QenloCollection *, uint64_t *);
char *qenlo_stats(QenloCollection *);
int32_t qenlo_export_qn(QenloCollection *, const char *);
int32_t qenlo_flush(QenloCollection *);
int32_t qenlo_close(QenloCollection *);
void qenlo_collection_free(QenloCollection *);
char *qenlo_last_error(void);
void qenlo_string_free(char *);
#endif
