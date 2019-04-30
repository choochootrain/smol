extern crate exitcode;
extern crate pest;
#[macro_use]
extern crate pest_derive;

use std::env;
use std::fs;
use std::io;
use std::io::prelude::*;

use pest::error::Error;
use pest::Parser;

use smol::result::{SmolError, SmolResult};

enum Input<'a> {
    Stdin,
    File(&'a str),
}

#[derive(Parser)]
#[grammar = "grammar/jsonish.pest"]
struct ESONParser;

#[derive(Debug, PartialEq)]
enum JSONValue<'a> {
    Object(Vec<(&'a str, JSONValue<'a>)>),
    Array(Vec<JSONValue<'a>>),
    String(&'a str),
    Number(f64),
    Boolean(bool),
    Null,
}

fn parse_jsonish_file(contents: &str) -> Result<JSONValue, Error<Rule>> {
    let jsonish = ESONParser::parse(Rule::jsonish, contents)?
        .next()
        .unwrap()
        .into_inner()
        .next()
        .unwrap();

    use pest::iterators::Pair;

    fn parse_value(pair: Pair<Rule>) -> JSONValue {
        match pair.as_rule() {
            Rule::object => JSONValue::Object(
                pair.into_inner()
                    .map(|pair| {
                        let mut inner_rules = pair.into_inner();

                        let name = inner_rules.next().unwrap();
                        let name = match name.as_rule() {
                            Rule::string => name
                                .into_inner()
                                .next()
                                .unwrap()
                                .into_inner()
                                .next()
                                .unwrap()
                                .as_str(),
                            Rule::identifier_name => name.as_str(),
                            _ => {
                                println!("UNEXPECTED {:?}", name);
                                unreachable!()
                            }
                        };

                        let value = parse_value(inner_rules.next().unwrap());
                        (name, value)
                    })
                    .collect(),
            ),
            Rule::array => JSONValue::Array(pair.into_inner().map(parse_value).collect()),
            Rule::string => JSONValue::String(
                pair.into_inner()
                    .next()
                    .unwrap()
                    .into_inner()
                    .next()
                    .unwrap()
                    .as_str(),
            ),
            Rule::number => JSONValue::Number(pair.as_str().parse().unwrap()),
            Rule::boolean => JSONValue::Boolean(pair.as_str().parse().unwrap()),
            Rule::null => JSONValue::Null,
            Rule::jsonish
            | Rule::EOI
            | Rule::pair
            | Rule::value
            | Rule::double_string
            | Rule::single_string
            | Rule::inner_double_string
            | Rule::inner_single_string
            | Rule::double_char
            | Rule::single_char
            | Rule::escape_char
            | Rule::identifier_name
            | Rule::identifier_start
            | Rule::identifier_part
            | Rule::unicode_escape_sequence
            | Rule::unicode_letter
            | Rule::WHITESPACE => unreachable!(),
        }
    }

    Ok(parse_value(jsonish))
}

fn serialize_jsonvalue(val: &JSONValue) -> String {
    match val {
        JSONValue::Object(o) => {
            let contents: Vec<_> = o
                .iter()
                .map(|(name, value)| format!("\"{}\": {}", name, serialize_jsonvalue(value)))
                .collect();
            format!("{{{}}}", contents.join(","))
        }
        JSONValue::Array(a) => {
            let contents: Vec<_> = a.iter().map(serialize_jsonvalue).collect();
            format!("[{}]", contents.join(","))
        }
        JSONValue::String(s) => format!("\"{}\"", s),
        JSONValue::Number(n) => format!("{}", n),
        JSONValue::Boolean(b) => format!("{}", b),
        JSONValue::Null => format!("null"),
    }
}

fn help(args: Vec<String>) -> SmolResult<()> {
    println!(
        "usage: {} [-i] [FILE]
    Parse into JSON accepting single quotes, unquoted object properties, and trailing commas.
    Use -i for stdin or read from FILE",
        args[0]
    );

    SmolError(exitcode::USAGE, None).into()
}

fn parse(input: Input) -> SmolResult<()> {
    let contents = match input {
        Input::File(name) => fs::read_to_string(name)
            .map_err(|_| SmolError(exitcode::NOINPUT, Some("Could not open file".to_string())))?,
        Input::Stdin => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf).map_err(|_| {
                SmolError(
                    exitcode::NOINPUT,
                    Some("Could not read from stdin".to_string()),
                )
            })?;
            buf
        }
    };

    let json: JSONValue = parse_jsonish_file(&contents)
        .map_err(|e| SmolError::from_err(exitcode::DATAERR, &e, "Could not parse file"))?;

    println!("{}", serialize_jsonvalue(&json));

    Ok(())
}

fn run(args: Vec<String>) -> SmolResult<()> {
    let file_name: Option<&str> = match args.len() {
        2 => Some(&args[1]),
        _ => return help(args).into(),
    };

    match file_name {
        Some("-i") => parse(Input::Stdin).into(),
        Some(name) => parse(Input::File(name)).into(),
        None => help(args).into(),
    }
}

fn main() {
    match run(env::args().collect()) {
        Ok(_) => ::std::process::exit(exitcode::OK),
        Err(SmolError(code, Some(message))) => {
            println!("{}", message);
            ::std::process::exit(code);
        }
        Err(SmolError(code, _)) => ::std::process::exit(code),
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq as pretty_assert_eq;

    use super::JSONValue::*;
    use super::*;

    #[test]
    fn parses_json() {
        let json = r#"
        {
            "a": "foobar",
            "b": "\"baz\"",
            "c": -12.43,
            "d": null,
            "$nested": {
                "anotherOne": true,
                "asdf": false
            },
            "_stuff": [1, 2, 3, null, {}, false, "asdf", []],
            "a.b!{}": "c",
        }
        "#;

        let expected = Object(vec![
            ("a", String("foobar")),
            ("b", String("\\\"baz\\\"")),
            ("c", Number(-12.43)),
            ("d", Null),
            (
                "$nested",
                Object(vec![
                    ("anotherOne", Boolean(true)),
                    ("asdf", Boolean(false)),
                ]),
            ),
            (
                "_stuff",
                Array(vec![
                    Number(1.0),
                    Number(2.0),
                    Number(3.0),
                    Null,
                    Object(vec![]),
                    Boolean(false),
                    String("asdf"),
                    Array(Vec::new()),
                ]),
            ),
            ("a.b!{}", String("c")),
        ]);

        let result = parse_jsonish_file(json);

        assert!(result.is_ok(), "{:?}", result);

        let result = result.unwrap();
        pretty_assert_eq!(result, expected);

        let serialized = serialize_jsonvalue(&result);
        let expected = r#"{"a": "foobar","b": "\"baz\"","c": -12.43,"d": null,"$nested": {"anotherOne": true,"asdf": false},"_stuff": [1,2,3,null,{},false,"asdf",[]],"a.b!{}": "c"}"#;
        pretty_assert_eq!(serialized, expected);
    }

    #[test]
    fn parses_ecmascript_object() {
        let es = r#"
        {
            a: "foobar",
            b: "\"baz\"",
            c: -12.43,
            'd': null,
            $nested: {
                anotherOne: true,
                asdf: false,
            },
            _stuff: [1, 2, 3, null, {}, false, 'asdf', [],],
            "a.b!{}": "c",
        }
        "#;

        let expected = Object(vec![
            ("a", String("foobar")),
            ("b", String("\\\"baz\\\"")),
            ("c", Number(-12.43)),
            ("d", Null),
            (
                "$nested",
                Object(vec![
                    ("anotherOne", Boolean(true)),
                    ("asdf", Boolean(false)),
                ]),
            ),
            (
                "_stuff",
                Array(vec![
                    Number(1.0),
                    Number(2.0),
                    Number(3.0),
                    Null,
                    Object(vec![]),
                    Boolean(false),
                    String("asdf"),
                    Array(Vec::new()),
                ]),
            ),
            ("a.b!{}", String("c")),
        ]);

        let result = parse_jsonish_file(es);

        assert!(result.is_ok(), "{:?}", result);

        let result = result.unwrap();
        pretty_assert_eq!(result, expected);

        let serialized = serialize_jsonvalue(&result);
        let expected = r#"{"a": "foobar","b": "\"baz\"","c": -12.43,"d": null,"$nested": {"anotherOne": true,"asdf": false},"_stuff": [1,2,3,null,{},false,"asdf",[]],"a.b!{}": "c"}"#;
        pretty_assert_eq!(serialized, expected);
    }
}
