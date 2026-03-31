# Cattle C Compiler  

(pronounced like seattle)

This is a hobby C compiler written in Rust, with the help of the wonderful book written by Nora Sandler. 

I plan to eventually implement the entire C17 standard, including typedefs and atomics. As of commit a3142a0,
the compiler can handle basically every statement/expression category other than typedefs and a few function specifiers.
  
Here's an example fibonnaci program it can compile just as a demonstration of its capabilities:

```c 
int putchar(int c);

int print_num_recursive(int n) {
    if (n < 10) goto print_single;

    print_num_recursive(n / 10);

print_single:
    putchar((n % 10) + 48);
    return 1; 
}

int print_next_fib(int limit) {
    static int a = 0;
    static int b = 1;
    static int count = 0;
    int next;

    if (count >= limit) goto finished;

    print_num_recursive(a);
    count = count + 1;

    if (count == limit) goto finished;

    putchar(44);
    putchar(32);

    next = a + b;
    a = b;
    b = next;

finished:
    return count;
}

int main() {
    int limit = 15;
    int current_count = 0;

    while (current_count < limit) {
        current_count = print_next_fib(limit);
    }

    putchar(10);
    return 0;
}
```

Feel free to play around with it! I also always appreciate advice and suggestions, but I would recommend you not make pull requests as this is a personal learning project.

