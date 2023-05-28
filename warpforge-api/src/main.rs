// Here's code that's approx 100% correct... but now I want it from a macro:
/*
enum MyEnum {
    FirstVariant { val: String },
    SecondVariant { val: String },
    ThirdVariant { val: String },
}

impl std::fmt::Display for MyEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MyEnum::FirstVariant { val } => write!(f, "first:{}", val),
            MyEnum::SecondVariant { val } => write!(f, "second:{}", val),
            MyEnum::ThirdVariant { val } => write!(f, "third:{}", val),
        }
    }
}

impl std::str::FromStr for MyEnum {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(());
        }
        let discrim = parts[0];
        let val = parts[1];
        match discrim {
            "first" => Ok(MyEnum::FirstVariant { val: val.to_string() }),
            "second" => Ok(MyEnum::SecondVariant { val: val.to_string() }),
            "third" => Ok(MyEnum::ThirdVariant { val: val.to_string() }),
            _ => Err(()),
        }
    }
}
*/

#[macro_use]
extern crate catverters;

// #[derive(Debug)]
// enum MyEnum {
//     #[catverters::strval(discriminant = "first:")]
//     First(FirstVariant),
//     #[catverters::strval(discriminant = "second:")]
//     Second(SecondVariant),
//     #[catverters::strval(discriminant = "third:")]
//     Third(ThirdVariant),
// }
// catverters::GenerateDisplayAndFromStr!(MyEnum);

#[derive(Debug, catverters::Stringoid)]
enum MyEnum {
    First(FirstVariant),
    Second(SecondVariant),
    Third(ThirdVariant),
    //#[discriminant = "override"]
}

#[derive(Debug, catverters::Stringoid)]
struct FirstVariant {
    val: String,
}

#[derive(Debug, catverters::Stringoid)]
struct SecondVariant {
    val: String,
}

#[derive(Debug, catverters::Stringoid)]
struct ThirdVariant {
    val: String,
}

fn main() {
    let my_enum = MyEnum::First(FirstVariant {
        val: "asdf".to_string(),
    });
    println!("Enum value: {:?}", my_enum);
    println!("Described: {:?}", my_enum.describe());
    println!("Magic?: {:?}", my_enum.to_string());

    let parsed_enum: Result<MyEnum, _> = "Second:hello".parse();
    println!("Parsed enum value: {:?}", parsed_enum);
    println!("Re-displayed enum value: {}", parsed_enum.unwrap());
}
