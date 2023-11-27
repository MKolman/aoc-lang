# Another Obscure Coding Language

Another Obscure Coding Language (AOCLang) is my attempt at creating a simple interpreted programing language in Rust.

My hope is that it will mature enough that I will be able to use it to solve Advent of Code 2023.

## Primes

This code prints out all prime numbers up to 100

```
isPrime = fn(n) {
    m = 2
    prime = 1
    while m * m <= n {
        if n % m == 0
            prime = 0
        m = m + 1
    }
    prime
}

n = 2
while n < 100 {
    if isPrime(n)
        print(n)
    n = n + 1
}
```

## Everything returns a value

In AOCLang everything returns a value. Assignments, if statements, loops, blocks, you name it.

### Arithmetic operations

The return value of an arithmeic operation is its result

```
> 3 + 1 * (2+1)
-> Int(6)
```

### Variable assignment

The return value of an assignment is the value assigned

```
> a = 12 * 5
-> Int(60)
```

So you can freely chain assignments

```
> a = b = 42
-> Int(42)
```

### Print

Print is a built-in keyword that outputs value to STDOUT and also returns whatever it prints. So you dan do things like

```
> b = print(a = 12)
12
-> Int(12)
```

### Boolean operators

Boolean values are represented as Int(0) and Int(1) in AOC and more precisely any number other than 0 is equivalent to "TRUE".

```
> 12 == 12
-> Int(1)

> 2 == 3
-> Int(0)

> 2 == 1 | 3 + 2 < 10 & 5 > 10 / 3
-> Int(1)
```

### Block scope

Using curly braces `{` and `}` you can define a new scope where you can scope variables. It can contain multiple expressions (i.e. lines of code) and will return the value of the last expression in the block.

```
> {
    a = 15
    a = a + 5
    a = a * a
}
-> Int(400)
```

Blocks are not bariers for variable access. You can access variables from parent scope as well as from child scope.

```
> a = 1
> {a = a + 1}
> print(a)
| 2
> {b = a}
> print(b)
| 2
```

### If statement

`if` keyword folowed by two expressions will return the value of the second if the first is non-zeo and zero otherwise.

```
> if 1 10
-> Int(10)

> if 0 10
-> Int(0)

> if 12 < 30 / 2 print(12*12)
| 144
-> Int(144)
```

The syntax might look a bit strange but if you separate it by a new line or if the second statement is a block we get the familiar form of:

```
> if 12 < 30 / 2
    print(12 * 12)
| 144
-> Int(144)

> if 12 < 30 / 2 {
    print(12 * 12)
    8 * 3
}
| 144
-> Int(24)
```

This means that you can also assign via if statement like

```
> x = if x >= 100 { x - 1 }
```

which will reduce the value of x by one if it's greater than 100 or change it to 0 otherwise.

### While loop

`while` keyword followed by two staments (condition and body) will keep executing the body until the statement is 0. It will then return the last value of the body.

```
x = 0
while x < 4 {
    print(x)
    x = x + 1
}
| 0
| 1
| 2
| 3
-> Int(4)
```

### Function

Unlike some other languages functions are considered normal values and are as such also normally assigned to variables using the `fn` keyword.

```
> isEven = fn(n) n % 2 == 0
> print(isEven(1))
| 0
> print(isEven(112434))
| 1
```

Here's a more complex example:

```
isPrime = fn(n) {
    m = 2
    prime = 1
    while prime & m * m <= n {
        if n % m == 0
            prime = 0
        m = m + 1
    }
    prime
}
```

There is no return statement (yet) so just like oter statement they return the value that their body returns i.e. the last value in there.

## Values

The number of built-it types is very limited. There are many types as defined below. Each variable is able to hold any value and can freely change its typing.

### Nil

Nil is the default value for all variables and is what it returned from an empty block. It canbe constructed using the `nil` keyword.

```
> print(nil)
| nil
-> Nil
```

### Int/Float

There are two number types. A signed 64 bit integer and a 64 bit float. Construct them with number literals and mutate it using arithemtic operations.

```
> print(16 * 10 - 5 * (6 - 2))
| 140
> print(12.0 / 3)
| 4.0
```

### Function

Functions are first-class citizens in AOC Lang. They are assigned to variables and can be freely passed around.

```
isOdd = fn(n) n % 2
isEven = fn(n, checkOdd) !checkOdd(n)
print(isEven(5, isOdd))
| 0
print(isEven(4, idOdd))
| 1
```

### Vector

Vectors are a collection of arbitrary other values. Vectors can be added with other vectors to be concatenated.
You can also multiply them with integers to repeat them. Use prefix `+` to get the length of the vector.

```
v = [12 + 5, fn(x) x+1, [1, 2, 3]]
print(v[1](10))
| 11
v[2][1] = 10
print(v[2])
| [1, 10, 3]
print(v[2] * 3)
| [1, 2, 3, 1, 2, 3, 1, 2, 3]
v = [nil] * 100
print(+v)
| 100
```

### Object

Objects are a collection of key-value pairs. They can be accessed using the square brackets.
The dot notation is just a syntactic sugar for accessing string keys.

```
author = {= "name"="Maks", "age"=25} # TODO inline definition
author.name = "Maks Kolman" # same as author["name"] = "Maks Kolman"
author["age"] = author.age + 5
author[69] = "Nice"
print(author)
| {=age: 30, name: Maks Kolman, 69: Nice}
```

#### Object builders

We can use objects as poor-mans classes by using object builder functions.
These function return an object with some public fields and methods attached to it.
You can always have private fields by defining local variables in the function body
and then using them in the methods.

```
Animal = fn(name, sound, legs) {
  # name, sound and legs are private values for Animal. To make
  # them public we would append them to the animal object below
  animal = {=}
  animal.speak = fn() print(name, " says: ", sound)
  anumal.walk = fn() print(name, " walks on ", legs, " legs")
  animal
}
roo = Animal("Kangaroo", "Woof", 2)
roo.speak()
| Kangaroo says: Woof
fox = Animal("Fox", "Wa-pa-pa-pa-pa-pa-pow!", 4)
fox.speak()
| Fox says: Wa-pa-pa-pa-pa-pa-pow!
fox.walk()
| Fox walks on 4 legs
```

### String

Strings are immutable and can be constructed using double quotes. You can use the square brackets to access individual characters or slices of the string.
Use prefix `+` to get the length of the string.

```
> a = "Hello World"
> print(a)
| Hello World
> print(a[0, 5])
| Hello
> print(+a)
| 11
```

NOTE: Characters are not a new type they're just integer numbers equivalent to charaters ascii
value and can be accessed in string using the square brackets or constructed using single quotes `'`.

```
> a = "123"
> print(a[0])
| 49
print('A')
| 65
```

## Language TODO list

- [x] Arithmetic operations
  - `1 + 3 / (2 - 10)`
- [x] Printing
  - `print(12 * 3)`
- [x] Variables
  - `a = 5 + a*a`
  - `a += 3`
  - `b = a *= 10`
- [x] Logical operations
  - `a <= 10 | 5 + b < a & 12 == b`
- [x] If statement
  - `if x != 0 {print(x)} else {print("Zero")}`
- [x] While loop
  - `while x > 0 {x = x - 1}`
- [x] For loop
  - `for i = 0; i < 10; i = i + 1 { print(i) }`
- [x] Function definition and calling
  - `isEven = fn(n) n % 2 == 0`
- [x] Clojures
  - `adder = fn(n) fn(x) x + n`
- [x] Vectors
  - `a = [1, 2, [5, 6]]`
  - `a[0] = a[1] + a[-1][1]`
  - `a = a + [9, 10]`
  - `[x, y] = [1, 2]`
  - Unpack `[x, y, [z, w]] = a`
  - Slices: `a[0, 2]` -> `[1, 2]`
  - TODO:
    - Append without making a copy: `a << 10?`
- [x] Strings
  - `a = "Hello world"`
  - `print(a)`
- [x] Comments
  - `# This is a comment`
  - `a = 3 # inline comment`
- [x] Object
  - `author = {=}`
  - `author[69] = "Nice"`
  - `author["name"] = "Maks"`
  - `author.name = "Maks" # equivalent to above`
  - TODO:
    - Inline defitions `author = {= "name": "Maks", "age": 25}`
- [x] Read a line from stdin
  - `a = read()`
- [x] Return
  - `return 12`
  - Can also be used at the top level to terminate the program
- [x] Error reporting during parsing, compilation and runtime
- [x] Imports
  - `use "path/to/file.aoc"`
- [ ] Break
- [ ] Standard library
