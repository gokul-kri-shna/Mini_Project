#include <stdio.h>
#include <stdlib.h>

// Idiom 6: Data-Dependent Runtime Ownership
// Ownership transfer depends on runtime conditional value (if flag) rather than static call-site.
void process_with_flag(int *ptr, int should_free) {
    if (should_free) {
        free(ptr); // Ownership transferred dynamically at runtime!
    } else {
        printf("Retaining pointer value: %d\n", *ptr);
    }
}

int main() {
    int *data = (int *)malloc(sizeof(int));
    *data = 99;
    process_with_flag(data, 1);
    return 0;
}
