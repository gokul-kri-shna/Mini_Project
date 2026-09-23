#include <stdio.h>

// Idiom 4: Overlapping Pointer-Arithmetic Slices
// Function signature looks safe, but offsets passed at call site overlap.
void combine_slices(int *dst, const int *src, int n) {
    for (int i = 0; i < n; i++) {
        dst[i] += src[i];
    }
}

int main() {
    int buffer[10] = {1, 2, 3, 4, 5, 6, 7, 8, 9, 10};
    // Call site where dst (buffer + 2) and src (buffer + 3) overlap in memory range!
    combine_slices(buffer + 2, buffer + 3, 4);
    printf("Result: %d, %d\n", buffer[2], buffer[3]);
    return 0;
}
