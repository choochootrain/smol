extern crate exitcode;
extern crate pest;
#[macro_use] extern crate pest_derive;

use std::env;
use std::fs;

use pest::Parser;
use pest::error::Error;

use smol::result::{SmolResult, SmolError};

#[derive(Parser)]
#[grammar = "grammar/eson.pest"]
struct ESONParser;

enum JSONValue<'a> {
    Object(Vec<(&'a str, JSONValue<'a>)>),
    Array(Vec<JSONValue<'a>>),
    String(&'a str),
    Number(f64),
    Boolean(bool),
    Null,
}

fn parse_eson_file(file: &str) -> Result<JSONValue, Error<Rule>> {
    let json = ESONParser::parse(Rule::json, file)?
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
                        let name = inner_rules
                            .next()
                            .unwrap()
                            .into_inner()
                            .next()
                            .unwrap()
                            .as_str();
                        let value = parse_value(inner_rules.next().unwrap());
                        (name, value)
                    })
                    .collect(),
            ),
            Rule::array => JSONValue::Array(pair.into_inner().map(parse_value).collect()),
            Rule::string => JSONValue::String(pair.into_inner().next().unwrap().as_str()),
            Rule::number => JSONValue::Number(pair.as_str().parse().unwrap()),
            Rule::boolean => JSONValue::Boolean(pair.as_str().parse().unwrap()),
            Rule::null => JSONValue::Null,
            Rule::json => {
                unreachable!("asdf");
            },
            Rule::EOI
            | Rule:: pair
            | Rule::value
            | Rule::inner
            | Rule::char
            | Rule::WHITESPACE => unreachable!(),
        }
    }

    Ok(parse_value(json))
}

fn serialize_jsonvalue(val: &JSONValue) -> String {
    match val {
        JSONValue::Object(o) => {
            let contents: Vec<_> = o
                .iter()
                .map(|(name, value)|
                     format!("\"{}\": {}", name, serialize_jsonvalue(value)))
                .collect();
            format!("{{{}}}", contents.join(","))
        },
        JSONValue::Array(a) => {
            let contents: Vec<_> = a.iter()
                .map(serialize_jsonvalue)
                .collect();
            format!("[{}]", contents.join(","))
        },
        JSONValue::String(s) => format!("\"{}\"", s),
        JSONValue::Number(n) => format!("{}", n),
        JSONValue::Boolean(b) => format!("{}", b),
        JSONValue::Null => format!("null"),
    }
}

fn help(args: Vec<String>) -> SmolResult<()> {
    println!(
        "usage: {} FILE
    Parse FILE into JSON.",
        args[0]
    );

    SmolError(exitcode::USAGE, None).into()
}

fn parse(name: &str) -> SmolResult<()> {
    let file = fs::read_to_string(name)
        .map_err(|_| SmolError(exitcode::NOINPUT, Some("Could not open file".to_string())))?;

    let json: JSONValue = parse_eson_file(&file)
        .map_err(|e| SmolError::from_err(exitcode::DATAERR, &e, "Could not parse file"))?;

    println!("{}", serialize_jsonvalue(&json));

    Ok(())
}

fn run(args: Vec<String>) -> SmolResult<()> {
    let file_name: Option<&str> = match args.len() {
        2 => Some(&args[1]),
        _ => None,
    };

    match file_name {
        Some(name) => parse(name).into(),
        None => help(args).into()
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
