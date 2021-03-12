//! Parsers for validating bash code contained in task blocks.

use crate::parse::prelude::*;
use crate::parse::util::{braces, comment, ident, line, parens, whitespace};
use combine::parser::char::alpha_num;
use combine::parser::range::recognize_with_value;

// TODO we could simplify a lot of this by just using recognize_with_value everywhere.
// All the parses just return Vec<&'a str>, and we wrap it all in a big recognize_with_value
// at the end to capture the full text.

fn no_vars(s: &str) -> (&str, Vec<&str>) {
    (s, Vec::with_capacity(0))
}

fn full_text_and_vars<'a>(
    (full_text, (_, parsed_vars)): (&'a str, (&'a str, Vec<&'a str>)),
) -> (&'a str, Vec<&'a str>) {
    (full_text, parsed_vars)
}

p! {
    single_quoted_content() -> &'a str, {
        recognize(
            skip_many(
                recognize(skip_many1(none_of("'".chars())))
                .or(string("\\'"))
            )
        )
    }
}

p! {
    single_quoted_string() -> &'a str, {
        recognize(
            char('\'').and(single_quoted_content()).and(char('\''))
        )
    }
}

p! {
    double_quoted_special_content() -> (&'a str, Vec<&'a str>), {
        choice!(
            variable_like(),
            single_quoted_string().map(|s| (s, Vec::with_capacity(0))),
            escaped_char().map(|s| (s, Vec::with_capacity(0)))
        )
    }
}

// this is sort of confusing: using recognize_with_value, we return a tuple containing
// the full text of the double-quoted content, plus the value of parsing the suffix.
// The suffix parse value is an Option containing vars from special content we encounter,
// and vars from the remaining quoted content (i.e. vars that have already been recursively
// merged). We merge these together and return them, along with the full text.
p! {
    double_quoted_content() -> (&'a str, Vec<&'a str>), {
        recognize_with_value(
            skip_many(none_of("$\"\\".chars()))
                .with(optional(double_quoted_special_content().and(double_quoted_content())))
        ).map(|(full_text, parsed_suffix)| {
            let mut vars = Vec::new();
            if let Some(((_, mut special_vars), (_, mut rest_vars))) = parsed_suffix {
                vars.append(&mut special_vars);
                vars.append(&mut rest_vars);
            }
            (full_text, vars)
        })
    }
}

p! {
    double_quoted_string() -> (&'a str, Vec<&'a str>), {
        recognize_with_value(
            char('"').with(double_quoted_content()).skip(char('"'))
        ).map(full_text_and_vars)
    }
}

p! {
    string_literal() -> (&'a str, Vec<&'a str>), {
        choice!(
            single_quoted_string().map(|s| (s, Vec::with_capacity(0))),
            double_quoted_string()
        )
    }
}

p! {
    escaped_char() -> &'a str, {
        recognize(char('\\').and(any()))
    }
}

p! {
    non_zero_int() -> &'a str, {
        recognize(
            one_of("123456789".chars()).and(skip_many(one_of("0123456789".chars())))
        )
    }
}

p! {
    internal_variable() -> &'a str, {
        recognize(
            char('$').and(one_of("*@#?-$!0_".chars()))
        ).or(recognize(
            char('$').and(non_zero_int())
        ))
    }
}

// TODO note that string indexing is dangerous, since it might fall outside a char boundary...
// but we're not expecting multibyte chars in variable names so not worrying too hard about it.

p! {
    simple_variable() -> (&'a str, &'a str), {
        recognize(char('$').and(ident()))
            .map(|var: &'a str| (var, &var[1..]))
    }
}

p! {
    braced_variable() -> (&'a str, &'a str), {
        recognize(char('$').and(braces(ident())))
            .map(|var: &'a str| {
                let len = var.len();
                (var, &var[2..len - 1])
            })
    }
}

// we don't bother trying to parse variables inside string manipulations, too messy
p! {
    string_manipulation() -> &'a str, {
        recognize(
            char('$').and(braces(skip_many1(none_of("}".chars()))))
        )
    }
}

// TODO I think string expansion is way broader than this, but this is what ducttape has...
p! {
    string_expansion() -> &'a str, {
        recognize(
            string("$'\\").and(skip_many1(alpha_num())).and(char('\''))
        )
    }
}

// an unescaped dollar sign, meaning a literal dollar sign.
p! {
    dollar_only() -> &'a str, {
        recognize(char('$').and(whitespace()))
    }
}

p! {
    parens_section() -> (&'a str, Vec<&'a str>), {
        recognize_with_value(parens(bash_block())).map(full_text_and_vars)
    }
}

p! {
    braces_section() -> (&'a str, Vec<&'a str>), {
        recognize_with_value(braces(bash_block())).map(full_text_and_vars)
    }
}

p! {
    command_sub() -> (&'a str, Vec<&'a str>), {
        recognize_with_value(
            char('$').with(parens_section())
        ).map(full_text_and_vars)
    }
}

p! {
    in_process_sub() -> (&'a str, Vec<&'a str>), {
        recognize_with_value(
            char('<').with(parens_section())
        ).map(full_text_and_vars)
    }
}

p! {
    out_process_sub() -> (&'a str, Vec<&'a str>), {
        recognize_with_value(
            char('>').with(parens_section())
        ).map(full_text_and_vars)
    }
}

p! {
    variable_like() -> (&'a str, Vec<&'a str>), {
        choice!(
            attempt(internal_variable().map(no_vars)),
            attempt(command_sub()),
            attempt(simple_variable().map(|(s, v)| (s, vec![v]))),
            attempt(braced_variable().map(|(s, v)| (s, vec![v]))),
            attempt(string_manipulation().map(no_vars)),
            attempt(string_expansion().map(no_vars)),
            dollar_only().map(no_vars)
        )
    }
}

// for now, we only allow 'EOF':
p! {
    heredoc_marker() -> &'a str, {
        string("EOF")
    }
}

// note: don't yet recognize vars inside of heredocs.
p! {
    heredoc() -> &'a str, {
        recognize(
            string("<<")
            .and(optional(char('-')))
            .and(heredoc_marker())
            .and(char('\n'))
            .and(skip_many(line(any())))
            // TODO should confirm that heredoc_marker is at the start of a line
            .and(heredoc_marker())
        )
    }
}

// any chunk of text we can be sure won't have any variables in it.
p! {
    code_blob() -> &'a str, {
        recognize(
            skip_many(none_of("{}()\"'#$<\\".chars()))
            .and(optional(
                // special case for allowing '<' when not part of '<<' or '<('
                char('<').and(none_of("<(".chars())).and(code_blob())
            ))
        )
    }
}

p! {
    non_blob_element() -> (&'a str, Vec<&'a str>), {
        choice!(
            escaped_char().map(no_vars),
            variable_like(),
            in_process_sub(),
            out_process_sub(),
            parens_section(),
            braces_section(),
            string_literal(),
            comment().map(no_vars),
            heredoc().map(no_vars)
        )
    }
}

p! {
    bash_block() -> (&'a str, Vec<&'a str>), {
        recognize_with_value(
            code_blob().with(optional(non_blob_element().and(bash_block())))
        ).map(|(full_text, parsed_suffix)| {
            let mut vars = Vec::new();
            if let Some(((_, mut non_blob_vars), (_, mut suffix_vars))) = parsed_suffix {
                vars.append(&mut non_blob_vars);
                vars.append(&mut suffix_vars);
            }
            (full_text, vars)
        })
    }
}

p! {
    bash_code() -> crate::ast::BashCode<'a>, {
        bash_block().map(|(text, vars)| crate::ast::BashCode {
            text, vars: vars.into_iter().collect(),
        })
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use combine::EasyParser;
    #[test]
    fn test_single_quoted() -> Result<()> {
        assert_eq!(
            "'hi it me'",
            super::single_quoted_string()
                .easy_parse("'hi it me'")
                .unwrap()
                .0
        );
        assert_eq!(
            "''",
            super::single_quoted_string().easy_parse("''").unwrap().0
        );
        Ok(())
    }
    #[test]
    fn test_double_quoted() -> Result<()> {
        assert_eq!(
            ("\"simple example\"", vec![]),
            super::double_quoted_string()
                .easy_parse("\"simple example\"")
                .unwrap()
                .0
        );
        assert_eq!(
            ("\"has a $variable\"", vec!["variable"]),
            super::double_quoted_string()
                .easy_parse("\"has a $variable\"")
                .unwrap()
                .0
        );
        Ok(())
    }
    #[test]
    fn test_variable() -> Result<()> {
        assert_eq!(
            ("$variable", "variable"),
            super::simple_variable().easy_parse("$variable").unwrap().0
        );
        assert_eq!(
            ("$variable", vec!["variable"]),
            super::variable_like().easy_parse("$variable").unwrap().0
        );
        Ok(())
    }
}
