#![feature(proc_macro)]

#[macro_use]
extern crate quote;
extern crate syn;
extern crate proc_macro;
extern crate serde_json as json;
#[macro_use]
extern crate failure;
extern crate itertools;
extern crate rayon;

use rayon::prelude::*;

use std::str::FromStr;
use std::collections::HashSet;
use std::ascii::AsciiExt;
use std::path::{Path, PathBuf};
use std::fs::{self, File, DirEntry};
use failure::Error;
use itertools::Itertools;

use syn::Token::{
    Literal  as TLiteral,
    FatArrow as TFatArrow,
    Ident    as TIdent,
    ModSep   as TModSep,
    Pound    as TPound
};
use syn::TokenTree::{
    self,
    Token     as TTToken,
    Delimited as TTDelimited
};
use syn::Lit::Str;
use syn::Ident;
use syn::MetaItem;
use syn::Delimited;
use syn::DelimToken::Bracket;
use quote::Tokens;
use proc_macro::TokenStream;
use json::Value;
use std::time::Instant;

#[proc_macro_derive(JsonTests, attributes(directory, test_with, bench_with, skip))]
pub fn json_tests(input: TokenStream) -> TokenStream {
    // Construct a string representation of the type definition
    let s = input.to_string();



    // Parse the string representation
    let ast = syn::parse_derive_input(&s).unwrap();

    // Build the impl
    let gen = match impl_json_tests(&ast) {
        Ok(tokens) => tokens,
        Err(err) => panic!("{}", err)
    };


    // Return the generated impl
    let parsed = gen.parse().unwrap();

    parsed
}

fn impl_json_tests(ast: &syn::DeriveInput) -> Result<quote::Tokens, Error> {
    let name = &ast.ident;
    let config = extract_attrs(&ast)?;
    let tests = read_tests_from_dir(&config.directory)?;
    let mut tokens = quote::Tokens::new();

    // split tests into groups by filepath
    let tests = tests.into_iter().group_by(|test| test.path.clone());

    // create identifiers
    let test_func_name = config.test_func.rsplit(":").next().unwrap();
    let test_func_path = Ident::from(config.test_func.as_ref());
    let test_func_name = Ident::from(test_func_name);

    open_directory_module(&config, &mut tokens);

    for (filepath, tests) in &tests {
        let tests = tests.collect::<Vec<_>>();

        // If tests count in this file is 1, we don't need submodule
        let need_file_submodule = tests.len() > 1;

        if need_file_submodule {
            open_file_module(&filepath, &mut tokens);
        }

        // Generate test function
        for test in tests {
            let name = sanitize_ident(&test.name);
            let name_ident = Ident::from(name.as_ref());
            let data = json::to_string(&test.data)?;

            // generate optional benchmark body
            if let Some(ref bench_func_path) = config.bench_func {
                // create identifiers
                let bench_func_name = bench_func_path.rsplit(":").next().unwrap();
                let bench_func_path = Ident::from(bench_func_path.as_ref());
                let bench_func_name = Ident::from(bench_func_name);

                let name = format!("bench_{}", name);
                let name_ident = Ident::from(name.as_ref());

                tokens.append(quote! {
                    #[bench]
                    fn #name_ident(b: &mut test::Bencher) {
                        use #bench_func_path;
                        let data = #data;
                        #bench_func_name(b, #name, data);
                    }
                })
            }

            // generate test body
            tokens.append(quote! {
                #[test]
                fn #name_ident() {
                    use #test_func_path;
                    let data = #data;
                    #test_func_name(#name, data);
                }
            });
        }

        if need_file_submodule {
            // Close file module
            close_brace(&mut tokens)
        }
    }

    // Close directory module
    close_brace(&mut tokens);

    Ok(tokens)
}

fn open_directory_module(config: &Config, tokens: &mut quote::Tokens) {
    // get the leaf directory name
    let dirname = config.directory.rsplit("/").next().unwrap();

    // create identifier
    let dirname = sanitize_ident(dirname);
    let dirname = Ident::from(dirname);

    open_module(dirname, tokens);
}

fn open_file_module(filepath: &str, tokens: &mut quote::Tokens) {
    // get file name without extension
    let filename = filepath.rsplit("/").next().unwrap()
                           .split(".").next().unwrap();
    // create identifier
    let filename = sanitize_ident(filename);
    let filename = Ident::from(filename);

    open_module(filename, tokens);
}

fn open_module(module_name: Ident, tokens: &mut quote::Tokens) {
    // append module opening tokens
    tokens.append(quote! {
        mod #module_name
    });
    tokens.append("{");
}

fn close_brace(tokens: &mut quote::Tokens) {
    tokens.append("}")
}

fn sanitize_ident(ident: &str) -> String {
    // replace empty ident
    let ident = if ident.is_empty() {
        String::from("unnamed")
    } else { ident.to_string() };

    // prepend alphabetic character if token starts with non-alphabetic character
    let ident = if ident.chars().nth(0).map(|c| !c.is_alphabetic()).unwrap_or(true) {
        format!("x_{}", ident)
    } else { ident };

    // replace special characters with _
    let ident = replace_chars(&ident, "!@#$%^&*-+=/<>;\'\"()`~", "_");

    ident
}

fn replace_chars(s: &str, from: &str, to: &str) -> String {
    let mut initial = s.to_owned();
    for c in from.chars() {
        initial = initial.replace(c, to);
    }
    initial
}

fn extract_attrs(ast: &syn::DeriveInput) -> Result<Config, Error> {
    const ERROR_MSG: &str = "expected 2 attributes and 1 optional\n\n\
                #[derive(JsonTests)]\n\
                #[directory = \"../tests/testset\"]\n\
                #[test_with = \"test::test_function\"]\n\
                #[bench_wuth = \"test::bench_function\"] (Optional)\n\
                struct TestSet;";

    if ast.attrs.len() < 2 || ast.attrs.len() > 3 {
        bail!(ERROR_MSG);
    }

    let config = ast.attrs.iter().fold(Config::default(), |config, attr| {
        match attr.value {
            MetaItem::NameValue(ref ident, Str(ref value, _)) => {
                match ident.as_ref() {
                    "directory" => Config { directory: value.clone(), ..config },
                    "test_with" => Config { test_func: value.clone(), ..config },
                    "bench_with" => Config { bench_func: Some(value.clone()), ..config },
                    _ => panic!("{}", ERROR_MSG),
                }
            },
            MetaItem::Word(ref ident) => {
                match ident.as_ref() {
                    "skip" => Config { skip: true, ..config },
                    _ => panic!("{}", ERROR_MSG),
                }
            }
            _ => panic!("{}", ERROR_MSG)
        }
    });

    Ok(config)
}

#[derive(Default)]
struct Config {
    directory: String,
    test_func: String,
    bench_func: Option<String>,
    skip: bool,
}

fn read_tests_from_dir<P: AsRef<Path>>(dir_path: P) -> Result<impl Iterator<Item=Test>, Error> {
    let dir = fs::read_dir(dir_path)?;

    let iter = dir.into_iter()
        .flat_map(|file|{
            match file {
                Ok(file) => tests_iterator_from_direntry(file).unwrap(),
                Err(err) => panic!("failed to read dir: {}", err)
            }
        });

    Ok(iter)
}

fn tests_iterator_from_direntry(file: DirEntry) -> Result<impl Iterator<Item=Test>, Error> {
    let path = file.path().to_owned();
    let file = File::open(&path)?;
    let mut tests: Value = json::from_reader(file)?;

    // Move out the root object
    let tests = match tests {
        Value::Object(tests) => tests,
        _ => panic!("expected a json object at the root of test file")
    };

    let iter = tests.into_iter().map(move |(name, data)| Test {
        path: path.to_str().unwrap().to_owned(),
        name,
        data
    });

    Ok(iter)
}

struct TestIterator {
    dir: DirEntry,

}

struct Test {
    path: String,
    name: String,
    data: Value
}
