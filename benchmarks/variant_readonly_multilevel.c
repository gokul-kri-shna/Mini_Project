int read_nested(int **data) {
    return **data;
}

int main() {
    int value = 42;
    int *inner = &value;
    int **outer = &inner;
    return read_nested(outer);
}
