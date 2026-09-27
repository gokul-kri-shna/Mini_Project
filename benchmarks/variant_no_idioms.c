int add(int a, int b) {
    return a + b;
}

int square(int x) {
    return x * x;
}

int main() {
    int result = add(3, 4);
    int sq = square(result);
    return sq;
}
