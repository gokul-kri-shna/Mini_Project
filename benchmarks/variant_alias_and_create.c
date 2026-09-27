#include <stdlib.h>

void make_record(int **slot, int val) {
    *slot = (int *)malloc(sizeof(int));
    **slot = val;
}

void merge_totals(int *total_x, int *total_y) {
    *total_x += 10;
    *total_y += 20;
}

int main() {
    int *record = NULL;
    make_record(&record, 7);

    int shared = 5;
    merge_totals(&shared, &shared);

    return 0;
}
