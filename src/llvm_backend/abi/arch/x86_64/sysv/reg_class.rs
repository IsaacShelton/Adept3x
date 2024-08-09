use derive_more::IsVariant;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, IsVariant)]
pub enum RegClass {
    #[default]
    NoClass,
    Integer,
    Sse,
    SseUp,
    X87,
    X87Up,
    ComplexX87,
    Memory,
}

impl RegClass {
    pub fn merge(self, other: Self) -> Self {
        use RegClass::{ComplexX87, Integer, Memory, NoClass, Sse, X87Up, X87};

        assert_ne!(self, Memory);
        assert_ne!(self, ComplexX87);

        if self == other || other == NoClass {
            return self;
        }

        if other == Memory {
            return Memory;
        }

        if self == NoClass {
            return other;
        }

        if self == Integer || other == Integer {
            return Integer;
        }

        if matches!(self, X87 | X87Up | ComplexX87) || matches!(other, X87 | X87Up) {
            return Memory;
        }

        Sse
    }
}
