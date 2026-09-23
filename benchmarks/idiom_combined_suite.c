#include <stdio.h>
#include <stdlib.h>

// Combined Test Benchmark Suite:
// Integrates Idioms 1, 2, 3, 5, and 6 in a multi-function C systems program.

union Payload {
    int secret_code;
    float ratio;
};

// Idiom 3: T** Out-Parameter Allocation
void allocate_payload(int **out, int initial_val) {
    *out = (int *)malloc(sizeof(int));
    **out = initial_val;
}

// Idiom 1: Aliased Mutable Parameters
void update_twin_counters(int *counter_a, int *counter_b) {
    *counter_a += 1;
    *counter_b += 5;
}

// Idiom 5: Union Variant Write
fn_set_union(union Payload *p, int code) {
    p->secret_code = code;
}

// Idiom 2 & 6: T** Release & Data-Dependent Runtime Control Flow
void cleanup_resource(int **ptr, int force_cleanup) {
    if (force_cleanup) {
        if (ptr && *ptr) {
            free(*ptr);
            *ptr = NULL;
        }
    }
}

int main() {
    int *handle = NULL;
    
    // Test Idiom 3: Create Ownership
    allocate_payload(&handle, 100);
    printf("Allocated handle value: %d\n", *handle);

    // Test Idiom 1: Aliased Parameters
    update_twin_counters(handle, handle);
    printf("Updated handle value: %d\n", *handle);

    // Test Idiom 2 & 6: Release & Data-Dependent Ownership
    cleanup_resource(&handle, 1);

    if (handle == NULL) {
        printf("Cleanup completed successfully!\n");
    }

    return 0;
}
