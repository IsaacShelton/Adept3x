
# thread safety (draft)

Two primary datatypes:

- `T` thread-safe value
- `unsync<T>` non-thread-safe value

Two parameter passing modes:

- Normal (`T` or `unsync<T>`)
- View (`&T`) which accepts either `T` or `unsync<T>`, but can't be kept around

Outline:

- Values are thread-safe by default
- Values that are not thread-safe are marked as `unsync<T>`
- `unsync<T>` values are not coerecd to `T` values
- However, both `T` and `unsync<T>` values coerce to `&T`, a type which only exists when passing a value as parameter and prevents the value from being kept around (also `&T` can't pass thread boundries as it would violate thread-safety)
- Some `unsync<T>` values may be explicitly converted to `T`
- Most of the time, mutability is not thread-safe and requires an `unsync<T>`
- `unsync<T>` values cannot be shared between threads without synchronization by
  the programmer
- `unsync<T>` types cannot be contained in `T` types, as this would violate thread-safety
- If `T` exists, `unsync<T>` must start with its contents
- If `unsync<T>` is not explicitly defined, then it is automatically implemented as allowing mutation of public fields and having ability to freeze into `T` via shallow clone.

Example of creating a mutable list and passing it to a function that temporarily
reads the data of the list.

```
func main {
    list := unsync<List<int>>::new()

    for i in Range::upto(10) {
        list.append(i)
    }

    printIntegers(list)
}

func printIntegers(integers &List<int>) {
    for i in Range::upto(integers.length()) {
        print(integers[i])
    }
}
```

Example of a basic string type.

```
struct String (#[private] { array ptr<u8>, length uint }) {
    func new() Self {
        return { array: null, length: 0 }
    }

    func fromRaw(array ptr<u8>, length uint) {
        return { array: _, length, _ }
    }

    func fromC(cstr ptr<u8>) Self {
        length := strlen(cstr)
        array := malloc(length)
        memcpy(array, cstr, length)
        return { array: _, length: _ }
    }

    #[elder] // To prevent the value from being dropped early if this is called
    func viewRawContent(&self) ptr<u8> {
        return self.array
    }

    func length(&self) uint {
        return self.length
    }

    #[impl Drop]
    func drop(&self) {
        free(self.array)
    }

    #[impl Add]
    func add(a, b &Self) Self {
        length := a.length() + b.length()
        array := malloc(sizeof u8 * length)
        memcpy(array, a.viewRawContent(), a.length())
        memcpy(&array[a.length()], b.viewRawContent(), b.length())
        return { array: _, length: _ }
    }
}

struct unsync<String> (..., #[private] capacity uint) {
    func new() Self {
        capacity := 16
        array := malloc(sizeof u8 * capacity)
        return { array: _, length: 0, capacity: _ }
    }

    func new(initial &String) Self {
        length := initial.length()
        anticipated_extra := 16
        capacity := length + anticipated_extra
        array := malloc(sizeof u8 * capacity)
        memcpy(array, initial.viewRawContent(), length)
        return { array: _, length: _, capacity: _ }
    }

    func append(self, other &String) {
        new_capacity := max(self.capacity, 16)

        while new_capacity < self.length() + other.length() {
            new_capacity *= 2
        }

        new_array := realloc(self.array, new_capacity)
        assert(new_array != null)

        memcpy(new_array, self.array, self.length)
        memcpy(&new_array[self.length], other.viewRawContent(), other.length())

        self.array = new_array
        self.length += other.length()
        self.capacity = new_capacity
    }

    func finalize(self) String {
        result := String::fromRaw(self.array, self.length)
        self.array = null
        self.length = 0
        self.capacity = 0
        return result
    }
}
```

Example of a basic primarily unsync type.

```
struct unsync<GameData> (
    entities unsync<SlotMap<EntityKey, unsync<Entity>>>,
    total_score int,
    scores unsync<List<int>>,
    should_close bool,
) {
    func new() Self {
        return {
            entities: unsync<SlotMap<EntityKey, unsync<Entity>>>::new(),
            total_score: 0,
            scores: unsync<List<int>>::new(),
            should_close: false,
        }
    }

    func addScore(self, score int) {
        self.scores.append(score)
    }

    func spawnEntity(self, entity unsync<Entity>) EntityKey {
        return self.entities.insert(entity)
    }

    func averageScore(self) double {
        avg := 0.0
        for score in self.scores {
            avg += score as double
        }
        avg /= self.scores.length() as double
        return avg
    }

    func step(self) {
        if self.entities.count() > 0 {
            self.spawnEntity(unsync<Entity>::new())
        }
    }
}

func main {
    gamedata := unsync<GameData>::new()

    until gamedata.should_close {
        gamedata.step()
    }
}
```

Example of hypothetical simple HTTP server.

```
func main {
    server := HttpServer::new()
    server.on("/", func &index)
    server.on("/headers", func &headers)
    server.listen()
}

func index(request HttpRequest, responder HttpResponder) {
    weather := fetch("https://api.weather.dev/today.json")

    weather := if JSON::parse(weather) is Some(weather) {
        weather
    } else {
        responder.status(::BAD_GATEWAY)
        responder.writeln("Failed to get weather data")
        return
    }

    responder.status(::OK)
    responder.writeln(weather.summary)
}

func headers(request HttpRequest, responder HttpResponder) {
    for header in request.headers {
        responder.println("{} = {}", header.key, header.value)
    }
}
```
