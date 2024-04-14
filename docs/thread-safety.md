
# thread safety (draft)

- Values are thread-safe by default
- Values that are not thread-safe are marked as `unsync<T>`
- `unsync<T>` values are coerced to `T` values automatically (they are borrowed when safe, otherwise cloned)
- Some `unsync<T>` values may be explicitly converted to `T` using `unsync::build(unsync<T>) T`
- Most of the time, mutability is not thread-safe and requires an `unsync<T>`
- `unsync<T>` values cannot be shared between threads without synchronization by the programmer

```
func main {
    list := unsync<List<int>>::new()

    for i in Range::upto(10) {
        list.append(i)
    }

    print(list)
}
```

```
func main {
    people := unsync<HashMap<String, int>>::new()

    people.insert("John", 23)
    people.insert("Jake", 38)

    name := unsync<String>::new()
    name.append("J")
    name.append("u")
    name.append("l")
    name.append("i")
    name.append("u")
    name.append("s")

    people.insert(name, 55)

    for i in Range::upto(10) {
        addPeopleTo(people)
    }

    for entry in people {
        fmt::println("{} has been alive for {} years.", entry.key, entry.value)
    }
}

func addPeopleTo(hashmap unsync<HashMap<String, int>>) {
    hashmap.insert("Alice", 100)
    hashmap.insert("Bob", 200)
    hashmap.insert("Charlie", 300)
}

struct GameData (
    entities unsync<SlotMap<EntityKey, unsync<Entity>>>,
    total_score int,
    scores unsync<List<int>>,
)

struct Entity (hp, damage int, position pod<Vector2f>, texture Texture)
```

```
#[pod]
struct Hash (value u64)
```

```
struct String (#[private] array ptr<u8>, #[private] metadata u64) {
    #[static]
    func new Self {
    	return { array: null, metadata: 0x8000000000000000 }
    }
    
    #[static]
    func new(contents String) Self {
        return contents
    }
    
    #[static]
    func fromNullTerminated(null_terminated ptr<u8>) Self {
        length := strlen(null_terminated)
        array := malloc(length + 1)
        memcpy(array, null_terminated, length + 1)
        return { array: array, metadata: length as u64 }
    }
    
    func length uint {
        return (self.metadata & 0x7fffffffffffffff) as uint
    }

    func hash Hash {
        // (implementation details)
    }
    
    #[private]
    func isStaticLifetime bool {
        return (self.metadata & 0x8000000000000000) as bool
    }
}

struct unsync<String> (..., #[private] capacity usize) {
    #[static]
    func new Self {
        return { array: null, metadata: 0, capacity: 0 }
    }
    
    func append(other String) {
        // (implementation details)
    }

    func append(other rune) {
        // (implementation details)
    }

    func append(others Iterator<String>) {
        // (implementation details)
    }
}

struct List<$T> (#[private] array ptr<$T>, #[private] metadata u64) {
    func length uint {
        self.metadata & 0x7fffffffffffffff
    }
}

struct unsync<List<$T>> (..., #[private] capacity u64) {
    func append(item $T) {
        // (implementation details)
    }

    func append(items Iterator<$T>) {
        // (implementation details)
    }

    func clear {
        self.metadata &= 0x8000000000000000
    }
}
```

