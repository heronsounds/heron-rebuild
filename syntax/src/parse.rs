use anyhow::Result;

#[derive(Debug, thiserror::Error)]
#[error("ParseError on line '{line}': {msg}")]
pub struct Error {
    msg: String,
    pos: usize,
    line: String,
}

pub fn parse(text: &str) -> Result<Vec<crate::ast::Item<'_>>> {
    use combine::EasyParser;
    tapefile::items()
        .easy_parse(text)
        .map(|(items, _remainder)| {
            // TODO do something w/ the remainder.
            items
        })
        .map_err(|e| {
            let pos = e.position.translate_position(text);
            // isolate the line in question:
            let before = &text[0..pos];
            let after = &text[pos..text.len()];
            let prefix: String = before.chars().rev().take_while(|&c| c != '\n').collect();
            let prefix: String = prefix.chars().rev().collect();
            let suffix: String = after.chars().take_while(|&c| c != '\n').collect();
            let line = prefix + &suffix;
            // since converting combine's errors is a lifetime nightmare,
            // we just stringify the error before returning it.
            Error {
                pos,
                line,
                msg: format!("{}", e),
            }
            .into()
        })
}

pub mod prelude {
    pub use combine::parser::char::{char, string};
    pub use combine::parser::range::recognize;
    pub use combine::*;
    // pub use crate::macros::{p, repeater, wrapper};
}

pub mod util {

    use super::prelude::*;
    use combine::parser::char::{alpha_num, letter, space};
    // use combine::parser::sequence::skip;

    p! {
        ident_start() -> char, {
            char('_').or(letter())
        }
    }

    p! {
        ident_rest() -> Vec<char>, {
            many(char('_').or(alpha_num()))
        }
    }

    p! {
        branch_ident_parts() -> Vec<char>, {
            many1(char('_').or(alpha_num()))
        }
    }

    // unlike other idents, branch idents can start w/ a number.
    p! {
        branch_ident() -> &'a str, {
            recognize(branch_ident_parts())
        }
    }

    // TODO should idents be limited to ascii?
    p! {
        ident() -> &'a str, {
            recognize(ident_start().and(ident_rest()))
        }
    }

    p! {
        comment() -> &'a str, {
            recognize(
                char('#')
                .and(skip_many(none_of("\n".chars())))
                .and(char('\n'))
            )
        }
    }

    p! {
        whitespace() -> (), {
            skip_many1(
                space().map(|_| ()).or(comment().map(|_| ()))
            )
        }
    }

    wrapper! {
        lex(parser), {
            optional(whitespace()).with(parser).skip(optional(whitespace()))
        }
    }

    p! {
        line_internal_whitespace() -> (), {
            skip_many1(satisfy(|c: char| c.is_whitespace() && c != '\n'))
        }
    }

    wrapper! {
        lex_inline(parser), {
            optional(line_internal_whitespace())
                .with(parser)
                .skip(optional(line_internal_whitespace()))
        }
    }

    // parser, followed by *mandatory* whitespace
    wrapper! {
        lex_word(parser), {
            optional(whitespace()).with(parser).skip(whitespace())
        }
    }

    // parser, followed by *mandatory* line-internal whitespace
    wrapper! {
        lex_word_inline(parser), {
            optional(line_internal_whitespace()).with(parser).skip(line_internal_whitespace())
        }
    }

    wrapper! {
        parens(parser), {
            char('(').with(parser).skip(char(')'))
        }
    }

    wrapper! {
        braces(parser), {
            char('{').with(parser).skip(char('}'))
        }
    }

    wrapper! {
        brackets(parser), {
            char('[').with(parser).skip(char(']'))
        }
    }

    p! {
        eol() -> (), {
            eof().or(char('\n').and(optional(whitespace())).map(|_| ()))
        }
    }

    wrapper! {
        line(parser), {
            lex_inline(parser).skip(eol())
        }
    }

    repeater! {
        comma_delim(parser), {
            sep_by1(lex(parser), char(','))
        }
    }

    #[cfg(test)]
    mod test {
        use anyhow::Result;
        use combine::parser::char::char;
        use combine::EasyParser;
        #[test]
        fn test_ident() -> Result<()> {
            assert_eq!("my_name", super::ident().easy_parse("my_name").unwrap().0);
            assert_eq!(
                "_start_under123",
                super::ident().easy_parse("_start_under123").unwrap().0
            );
            assert!(super::ident().easy_parse("1name").is_err());
            Ok(())
        }
        #[test]
        fn test_whitespace() -> Result<()> {
            assert_eq!(
                ((), "and more"),
                super::whitespace().easy_parse(" and more").unwrap()
            );
            assert_eq!(
                ((), "and text"),
                super::whitespace().easy_parse(" \n    and text").unwrap()
            );
            assert!(super::whitespace().easy_parse("x").is_err());
            Ok(())
        }
        #[test]
        fn test_lex() -> Result<()> {
            assert_eq!(
                'x',
                super::lex(combine::parser::char::char('x')).easy_parse("  x  ").unwrap().0
            );
            assert_eq!(
                'x',
                super::lex(combine::parser::char::char('x')).easy_parse(" \n x \n ").unwrap().0
            );
            Ok(())
        }
        #[test]
        fn test_lex_inline() -> Result<()> {
            assert_eq!(
                'x',
                super::lex_inline(char('x')).easy_parse("  x  ").unwrap().0
            );
            assert!(super::lex_inline(char('x')).easy_parse("\nx").is_err());
            assert_eq!(
                ('x', "\n"),
                super::lex_inline(char('x')).easy_parse("x\n").unwrap()
            );
            Ok(())
        }
        #[test]
        fn test_eol() -> Result<()> {
            assert_eq!(
                ((), "other stuff"),
                super::eol().easy_parse("\n  \n   other stuff").unwrap()
            );
            assert_eq!(((), ""), super::eol().easy_parse("").unwrap());
            Ok(())
        }
        #[test]
        fn test_line() -> Result<()> {
            assert_eq!('x', super::line(char('x')).easy_parse(" x").unwrap().0);
            assert_eq!('x', super::line(char('x')).easy_parse(" x\n").unwrap().0);
            Ok(())
        }
    }
}

mod literal {

    use super::prelude::*;

    const FORBID_UNQUOTED: [char; 11] = ['(', ')', '[', ']', '*', '@', '$', '+', '#', '"', '\''];

    wrapper! {
        double_quotes(parser), {
            char('"').with(parser).skip(char('"'))
        }
    }

    p! {
        double_quoted_literal() -> &'a str, {
            double_quotes(recognize(skip_many(none_of("\"".chars()))))
        }
    }

    p! {
        unquoted_literal_char() -> char, {
            satisfy(|c: char|
                !c.is_whitespace() && !FORBID_UNQUOTED.iter().any(|&forbidden| forbidden == c)
            )
        }
    }

    p! {
        unquoted_literal() -> &'a str, {
            recognize(skip_many1(unquoted_literal_char()))
        }
    }

    p! {
        literal() -> &'a str, {
            double_quoted_literal().or(unquoted_literal())
        }
    }

    p! {
        interp_literal() -> (&'a str, Vec<&'a str>), {
            super::interp::double_quoted_interp_string()
                .or(unquoted_literal().map(|s| (s, Vec::with_capacity(0))))
        }
    }

    #[cfg(test)]
    mod test {
        use anyhow::Result;
        use combine::EasyParser;
        #[test]
        fn test_literal() -> Result<()> {
            assert_eq!(
                "just_ident",
                super::literal().easy_parse("just_ident").unwrap().0
            );
            assert_eq!(
                "quoted text",
                super::literal().easy_parse("\"quoted text\"").unwrap().0
            );
            assert_eq!(
                "not greedy",
                super::literal().easy_parse("\"not greedy\" won't parse this").unwrap().0
            );
            assert_eq!(
                "filenames.are.ok",
                super::literal().easy_parse("filenames.are.ok").unwrap().0
            );
            Ok(())
        }
    }
}

mod interp {
    use super::prelude::*;
    use super::rhs::variable;
    use combine::parser::range::recognize_with_value;

    p! {
        interp_variable() -> (&'a str, Vec<&'a str>), {
            variable().map(|var| (var, vec![var]))
        }
    }

    p! {
        interp_content() -> (&'a str, Vec<&'a str>), {
            recognize_with_value(
                skip_many(none_of("$\"\\".chars()))
                    .with(optional(variable().and(interp_content())))
            ).map(|(full_text, parsed_suffix)| {
                if let Some((var, (_, mut rest_vars))) = parsed_suffix {
                    rest_vars.push(var);
                    (full_text, rest_vars)
                } else {
                    (full_text, Vec::with_capacity(0))
                }
            })
        }
    }

    p! {
        double_quoted_interp_string() -> (&'a str, Vec<&'a str>), {
            super::literal::double_quotes(interp_content())
        }
    }
}

mod graft {

    use super::prelude::*;
    use super::util::{brackets, branch_ident, comma_delim, ident, lex_inline};

    p! {
        branch_element() -> (&'a str, &'a str), {
            ident().skip(char(':')).and(lex_inline(branch_ident()))
        }
    }

    p! {
        branch_graft() -> Vec<(&'a str, &'a str)>, {
            brackets(comma_delim(branch_element()))
        }
    }

    #[cfg(test)]
    mod test {
        use anyhow::Result;
        use combine::EasyParser;
        #[test]
        fn test_branch_graft() -> Result<()> {
            assert_eq!(
                vec![("Branchpoint1", "val1"), ("Branchpoint2", "val2")],
                super::branch_graft()
                    .easy_parse("[Branchpoint1: val1, Branchpoint2: val2]")
                    .unwrap()
                    .0
            );
            // make sure newlines work:
            assert_eq!(
                vec![("Bp1", "val1"), ("Bp2", "val2"), ("Bp3", "val3")],
                super::branch_graft()
                    .easy_parse("[\n\tBp1: val1,\n\tBp2: val2 ,\nBp3: val3\n]")
                    .unwrap()
                    .0
            );
            Ok(())
        }
    }
}

mod rhs {

    use super::graft::branch_graft;
    use super::literal::{interp_literal, literal};
    use super::prelude::*;
    use super::util::{branch_ident, ident, lex_inline, parens, whitespace};
    use crate::ast::Rhs;

    p! {
        shorthand_variable() -> char, {
            // skip_count(1, char('@'))
            char('@')
        }
    }

    p! {
        variable() -> &'a str, {
            char('$').with(ident())
        }
    }

    p! {
        task_output() -> (&'a str, &'a str), {
            variable().and(char('@').with(ident()))
        }
    }

    p! {
        shorthand_task_output() -> &'a str, {
            char('@').with(ident())
        }
    }

    p! {
        grafted_variable() -> (&'a str, Vec<(&'a str, &'a str)>), {
            variable().and(branch_graft())
        }
    }

    p! {
        grafted_task_output() -> ((&'a str, &'a str), Vec<(&'a str, &'a str)>), {
            task_output().and(branch_graft())
        }
    }

    p! {
        shorthand_grafted_task_output() -> (&'a str, Vec<(&'a str, &'a str)>), {
            shorthand_task_output().and(branch_graft())
        }
    }

    p! {
        branchpoint_assignment() -> (&'a str, Rhs<'a>), {
            branch_ident().and(
                choice!(
                    attempt(lex_inline(char('=')).with(rhs())),
                    produce(|| Rhs::Unbound)
                )
            )
        }
    }

    p! {
        branchpoint_assignments() -> Vec<(&'a str, Rhs<'a>)>, {
            // TODO this is prob a dumb way to do this, but cdn't think of
            // anything else - we try to sep_by1 a whitespace-separated list
            // of branch assignments, and if that fails, we call sep_end_by1
            // to catch the trailing whitespace.
            attempt(
                sep_by1(branchpoint_assignment(), whitespace())
            ).or(
                sep_end_by1(branchpoint_assignment(), whitespace())
            )
            // many1(branchpoint_assignment())
        }
    }

    p! {
        branchpoint_prefix() -> &'a str, {
            ident().skip(lex_inline(char(':')))
        }
    }

    p! {
        branchpoint() -> (&'a str, Vec<(&'a str, Rhs<'a>)>), {
            parens(
                optional(whitespace())
                    .with(branchpoint_prefix())
                    .skip(optional(whitespace()))
                    .and(branchpoint_assignments())
                    .skip(optional(whitespace()))
            )
        }
    }

    p! {
        rhs() -> Rhs<'a>, {
            choice!(
                branchpoint().map(|(branchpoint, vals)| Rhs::Branchpoint { branchpoint, vals }),
                attempt(
                    shorthand_grafted_task_output()
                        .map(|(task, branch)| Rhs::ShorthandGraftedTaskOutput { task, branch })
                ),
                attempt(
                    shorthand_task_output()
                        .map(|task| Rhs::ShorthandTaskOutput { task })
                ),
                shorthand_variable().map(|_| Rhs::ShorthandVariable),
                attempt(
                    grafted_variable()
                        .map(|(name, branch)| Rhs::GraftedVariable { name, branch })
                ),
                attempt(
                    grafted_task_output()
                        .map(|((output, task), branch)| Rhs::GraftedTaskOutput{output, task, branch}),
                ),
                attempt(
                    task_output()
                        .map(|(output, task)| Rhs::TaskOutput {output, task})
                ),
                attempt(
                    interp_literal().map(|(text, vars)| {
                        if vars.is_empty() {
                            Rhs::Literal { val: text }
                        } else {
                            Rhs::Interp { text, vars }
                        }
                    })
                ),
                variable().map(|name| Rhs::Variable { name }),
                // nb: with interp_literal enabled, this will never execute:
                literal().map(|val| Rhs::Literal { val })
            )
        }
    }

    #[cfg(test)]
    mod test {
        use crate::ast::Rhs;
        use anyhow::Result;
        use combine::EasyParser;
        #[test]
        fn test_literal() -> Result<()> {
            assert_eq!(Rhs::literal("hi"), super::rhs().easy_parse("hi").unwrap().0);
            assert_eq!(
                Rhs::literal("hi"),
                super::rhs().easy_parse("\"hi\"").unwrap().0
            );
            Ok(())
        }
        #[test]
        fn test_variable() -> Result<()> {
            assert_eq!(
                Rhs::ShorthandVariable,
                super::rhs().easy_parse("@").unwrap().0
            );
            assert_eq!(
                Rhs::variable("var"),
                super::rhs().easy_parse("$var").unwrap().0
            );
            assert_eq!(
                Rhs::grafted_variable("var", vec![("Bp1", "val1")]),
                super::rhs().easy_parse("$var[Bp1: val1]").unwrap().0,
            );
            Ok(())
        }
        #[test]
        fn test_task_output() -> Result<()> {
            assert_eq!(
                Rhs::shorthand_grafted_task_output("task", vec![("Bp1", "val1")]),
                super::rhs().easy_parse("@task[Bp1:val1]").unwrap().0
            );
            assert_eq!(
                Rhs::shorthand_task_output("task"),
                super::rhs().easy_parse("@task").unwrap().0
            );
            assert_eq!(
                Rhs::task_output("output", "task"),
                super::rhs().easy_parse("$output@task").unwrap().0
            );
            assert_eq!(
                Rhs::grafted_task_output("output", "task", vec![("Bp1", "val1")]),
                super::rhs().easy_parse("$output@task[Bp1: val1]").unwrap().0
            );
            Ok(())
        }
        #[test]
        fn test_branchpoint() -> Result<()> {
            assert_eq!(
                ("val1", Rhs::literal("yes")),
                super::branchpoint_assignment().easy_parse("val1=yes").unwrap().0,
            );
            assert_eq!(
                Rhs::branchpoint(
                    "Bp1",
                    vec![("val1", Rhs::literal("yes")), ("val2", Rhs::literal("no"))],
                ),
                super::rhs().easy_parse("(Bp1: val1=yes val2=no)").unwrap().0
            );
            // make sure we can deal with multiline branchpoint assignments:
            assert_eq!(
                Rhs::branchpoint(
                    "Bp1",
                    vec![("val1", Rhs::literal("yes")), ("val2", Rhs::literal("no"))],
                ),
                super::rhs().easy_parse("(\nBp1:\n  val1=yes\n  val2=no\n)").unwrap().0
            );
            assert_eq!(
                Rhs::branchpoint("Bp1", vec![("a", Rhs::Unbound), ("b", Rhs::Unbound)],),
                super::rhs().easy_parse("(Bp1: a b)").unwrap().0
            );
            assert_eq!(
                Rhs::branchpoint("Bp1", vec![("a", Rhs::Unbound), ("b", Rhs::Unbound)],),
                super::rhs().easy_parse("(Bp1:\n a b)").unwrap().0
            );
            // ending assignments w/ whitespace:
            assert_eq!(
                Rhs::branchpoint("Bp1", vec![("a", Rhs::Unbound), ("b", Rhs::Unbound)],),
                super::rhs().easy_parse("(Bp1: a b )").unwrap().0
            );
            Ok(())
        }
    }
}

mod assignment {

    use super::prelude::*;
    use super::rhs::rhs;
    use super::util::{ident, lex_inline, line_internal_whitespace};
    use crate::ast::Rhs;

    p! {
        assignment() -> (&'a str, Rhs<'a>), {
            ident().and(
                choice!(
                    attempt(lex_inline(char('=')).with(rhs())),
                    optional(line_internal_whitespace()).map(|_| Rhs::Unbound)
                )
            )
        }
    }

    p! {
        dot_assignment() -> (&'a str, Rhs<'a>), {
            char('.').with(ident()).and(
                choice!(
                    attempt(lex_inline(char('=')).with(rhs())),
                    line_internal_whitespace().map(|()| Rhs::Unbound)
                )
            )
        }
    }

    #[cfg(test)]
    mod test {
        use crate::ast::Rhs;
        use anyhow::Result;
        use combine::EasyParser;
        #[test]
        fn test_unbound() -> Result<()> {
            assert_eq!(
                ("var", Rhs::Unbound),
                super::assignment().easy_parse("var  ").unwrap().0
            );
            Ok(())
        }
        #[test]
        fn test_regular_bound_assignment() -> Result<()> {
            assert_eq!(
                ("var", Rhs::literal("value")),
                super::assignment().easy_parse("var=value").unwrap().0
            );
            Ok(())
        }
        #[test]
        fn test_dot_assignment() -> Result<()> {
            assert_eq!(
                ("param", Rhs::literal("value")),
                super::dot_assignment().easy_parse(".param=value").unwrap().0
            );
            Ok(())
        }
        #[test]
        fn test_branched() -> Result<()> {
            assert_eq!(
                (
                    "var",
                    Rhs::branchpoint(
                        "Branchpt",
                        vec![("a1", Rhs::literal("a")), ("b2", Rhs::literal("b"))]
                    )
                ),
                super::assignment().easy_parse("var=(Branchpt: a1=a b2=b)").unwrap().0
            );
            Ok(())
        }
        #[test]
        fn test_branched_shorthand() -> Result<()> {
            assert_eq!(
                (
                    "var",
                    Rhs::branchpoint("Branchpt", vec![("a", Rhs::Unbound), ("b", Rhs::Unbound)])
                ),
                super::assignment().easy_parse("var=(Branchpt: a b)").unwrap().0
            );
            Ok(())
        }
        // // in DT, I think a grafted glob produces a space-separated list,
        // // but presumably it only works for a single branchpoint.
        // #[test]
        // fn test_graft_shorthand_glob() -> Result<()> {
        //     assert_eq!(
        //         (
        //             "dataset_json",
        //             Rhs::ShorthandGraftedTaskOutput {
        //                 task: "DumpHFDataset",
        //                 branch: vec![("Dataset", "*")],
        //             }
        //         ),
        //         super::assignment()
        //             .easy_parse("dataset_json=@DumpHFDataset[Dataset:*]")
        //             .unwrap()
        //             .0
        //     );
        //     Ok(())
        // }
    }
}

mod spec {

    use super::assignment::{assignment, dot_assignment};
    use super::prelude::*;
    use super::util::{ident, lex, lex_inline};
    use crate::ast::BlockSpec;

    p! {
        input_chunk() -> Vec<BlockSpec<'a>>, {
            lex_inline(char('<')).with(many(
                lex_inline(assignment()).map(|(lhs, rhs)| BlockSpec::Input{lhs, rhs})
            ))
        }
    }

    p! {
        output_chunk() -> Vec<BlockSpec<'a>>, {
            lex_inline(char('>')).with(many(
                lex_inline(assignment()).map(|(lhs, rhs)| BlockSpec::Output{lhs, rhs})
            ))
        }
    }

    p! {
        param_assignment() -> BlockSpec<'a>, {
            // special case since params can start with '.':
            choice! (
                assignment().map(|(lhs, rhs)| BlockSpec::Param{lhs, rhs, dot: false}),
                dot_assignment().map(|(lhs, rhs)| BlockSpec::Param{lhs, rhs, dot: true})
            )
        }
    }

    p! {
        param_chunk() -> Vec<BlockSpec<'a>>, {
            lex_inline(string("::"))
                .with(many(lex_inline(param_assignment())))
        }
    }

    // p! {
    //     package_chunk() -> Vec<BlockSpec<'a>>, {
    //         lex_inline(char(':')).with(many(
    //             lex_inline(ident()).map(|name| BlockSpec::Package{name})
    //         ))
    //     }
    // }

    p! {
        module_chunk() -> Vec<BlockSpec<'a>>, {
            lex_inline(
                char('@').with(ident())
            ).map(|name| {
                vec![BlockSpec::Module { name }]
            })
        }
    }

    p! {
        spec_chunk() -> Vec<BlockSpec<'a>>, {
            choice!(
                attempt(input_chunk()),
                attempt(output_chunk()),
                attempt(param_chunk()),
                module_chunk()
                // package_chunk()
            )
        }
    }

    p! {
        specs() -> Vec<BlockSpec<'a>>, {
            many(lex(spec_chunk()))
                .map(|mut vecs: Vec<Vec<BlockSpec<'a>>>| {
                    // TODO there's gotta be a better way, but combine is confusin.
                    let mut flattened = Vec::new();
                    for vec in &mut vecs {
                        flattened.append(vec);
                    }
                    flattened
                })
        }
    }

    #[cfg(test)]
    mod test {
        use crate::ast::{BlockSpec, Rhs};
        use anyhow::Result;
        use combine::EasyParser;
        #[test]
        fn test_specs() -> Result<()> {
            assert_eq!(
                vec![
                    BlockSpec::output("output", Rhs::literal("filename.tgz")),
                    BlockSpec::input("input1", Rhs::task_output("output", "task")),
                    // BlockSpec::package("package_name"),
                    BlockSpec::param("param1", Rhs::variable("var")),
                    BlockSpec::dot_param("param2", Rhs::literal("value")),
                ],
                super::specs().easy_parse(
                    "> output=filename.tgz < input1=$output@task \n:: param1=$var .param2=value"
                ).unwrap().0
            );
            Ok(())
        }
        #[test]
        fn test_params() -> Result<()> {
            assert_eq!(
                vec![BlockSpec::param("param1", Rhs::Unbound)],
                super::param_chunk().easy_parse(":: param1").unwrap().0
            );
            assert_eq!(
                vec![BlockSpec::param("param1", Rhs::Unbound)],
                super::spec_chunk().easy_parse(":: param1").unwrap().0
            );
            Ok(())
        }
    }
}

mod tasklike {
    use super::prelude::*;
    use super::spec::specs;
    use super::util::{braces, ident, lex_inline};
    use crate::ast::{BlockType, TasklikeBlock};
    use crate::bash::bash_code;

    p! {
        block_name(keyword: &'static str) -> &'a str, {
            lex_inline(string(keyword)).with(ident())
        }
    }

    p! {
        tasklike_block(keyword: &'static str, subtype: BlockType) -> TasklikeBlock<'a>, {
            block_name(keyword)
                .and(specs())
                .and(braces(bash_code()))
                .map(|((name, specs), code)| {
                    TasklikeBlock {
                        name,
                        subtype: *subtype,
                        specs,
                        code,
                    }
                })

        }
    }

    p! {
        task() -> TasklikeBlock<'a>, {
            tasklike_block("task", BlockType::Task)
        }
    }

    // p! {
    //     package() -> TasklikeBlock<'a>, {
    //         tasklike_block("package", BlockType::Package)
    //     }
    // }

    #[cfg(test)]
    mod test {
        use anyhow::Result;
        use combine::EasyParser;
        // use crate::HashSet;
        // use crate::ast::{TasklikeBlock, BlockSpec, BlockType, BashCode};
        #[test]
        fn test_task() -> Result<()> {
            assert_eq!(
                "task_name",
                super::block_name("task").easy_parse("task task_name").unwrap().0
            );
            // assert_eq!(
            //     TasklikeBlock {
            //         name: "task_name",
            //         subtype: BlockType::Task,
            //         specs: vec![BlockSpec::package("package_name")],
            //         code: BashCode {
            //             code: "echo 'hi'",
            //             vars: HashSet::default(),
            //         }
            //     },
            //     super::task().easy_parse(
            //         "task task_name\n  : package_name\n{\n  echo 'hi'\n}"
            //     ).unwrap().0
            // );
            Ok(())
        }
    }
}

mod grouplike {
    use super::prelude::*;
    use super::spec::specs;
    use super::tasklike::{block_name, tasklike_block};
    use super::util::{braces, whitespace};
    use crate::ast::{BlockType, GrouplikeBlock};

    p! {
        grouplike_block(
            keyword: &'static str,
            subtype: BlockType,
            internal_keyword: &'static str,
            internal_subtype: BlockType
        ) -> GrouplikeBlock<'a>, {
            block_name(keyword)
                .and(specs())
                .and(braces(
                    sep_by(tasklike_block(internal_keyword, *internal_subtype), whitespace())
                ))
                .map(|((name, specs), blocks)| {
                    GrouplikeBlock {
                        name,
                        subtype: *subtype,
                        specs,
                        blocks,
                    }
                })
        }
    }

    // p! {
    //     versioner() -> GrouplikeBlock<'a>, {
    //         grouplike_block(
    //             "versioner",
    //             BlockType::Versioner,
    //             "action",
    //             BlockType::Action,
    //         )
    //     }
    // }
}

mod config {
    use super::assignment::assignment;
    use super::prelude::*;
    use super::util::{braces, lex, line, whitespace};
    use crate::ast::Rhs;

    p! {
        global_config() -> Vec<(&'a str, Rhs<'a>)>, {
            lex(string("global")).with(braces(
                optional(whitespace()).with(
                    many(line(assignment()))
                )
            ))
        }
    }
}

mod plan {
    use super::prelude::*;
    use super::util::{
        braces, branch_ident, comma_delim, ident, lex, lex_inline, parens, whitespace,
    };
    use crate::ast::{Branches, CrossProduct, Plan};

    p! {
        branches() -> Branches<'a>, {
            char('*').map(|_| Branches::Glob).or(
                many1(lex(branch_ident()))
                .map(Branches::Specified)
            )
        }
    }

    p! {
        branch_selection() -> (&'a str, Branches<'a>), {
            parens(
                lex(ident()).skip(lex(char(':'))).and(branches())
            )
        }
    }

    p! {
        branch_selections() -> Vec<(&'a str, Branches<'a>)>, {
            lex(string("via")).with(sep_by1(branch_selection(), attempt(lex(char('*')))))
        }
    }

    p! {
        cross_product() -> CrossProduct<'a>, {
            lex(string("reach"))
                .with(comma_delim(ident()))
                .and(optional(branch_selections()))
                .map(|(goals, branches)| {
                    let branches = branches.unwrap_or_default();
                    CrossProduct { goals, branches }
                })
        }
    }

    p! {
        plan() -> Plan<'a>, {
            lex_inline(string("plan")).with(ident())
                .skip(whitespace())
                .and(braces(
                    many(lex(cross_product()))
                ))
                .map(|(name, cross_products)| Plan { name, cross_products })
        }
    }

    #[cfg(test)]
    mod test {
        // use anyhow::Result;
        use super::*;
        use combine::EasyParser;
        #[test]
        fn test_cross_product() {
            assert_eq!(
                CrossProduct {
                    goals: vec!["task"],
                    branches: vec![],
                },
                cross_product().easy_parse("reach task").unwrap().0
            );
        }
        #[test]
        fn test_plan() {
            assert_eq!(
                Plan {
                    name: "plan",
                    cross_products: vec![CrossProduct {
                        goals: vec!["task"],
                        branches: vec![],
                    }],
                },
                plan().easy_parse("plan plan {\n  reach task\n}").unwrap().0
            );
        }
        #[test]
        fn test_branches() {
            assert_eq!(Branches::Glob, branches().easy_parse("*").unwrap().0);
            assert_eq!(
                Branches::Specified(vec!["val"]),
                branches().easy_parse("val").unwrap().0
            );
            assert_eq!(
                Branches::Specified(vec!["v1", "v2"]),
                branches().easy_parse("v1 v2").unwrap().0
            );
            // TODO add more here to test full plan syntax
        }
    }
}

mod misc {
    use super::assignment::assignment;
    use super::literal::literal;
    use super::prelude::*;
    use super::util::{lex_inline, line};
    use crate::ast::Rhs;

    p! {
        import_statement() -> &'a str, {
            line(
                lex_inline(string("import")).with(literal())
            )
        }
    }

    p! {
        module_statement() -> (&'a str, Rhs<'a>), {
            line(
                lex_inline(string("module")).with(assignment())
            )
        }
    }

    #[cfg(test)]
    mod test {
        use anyhow::Result;
        use combine::EasyParser;
        // use crate::HashSet;
        // use crate::ast::Item;
        #[test]
        fn test_import() -> Result<()> {
            assert_eq!(
                "packages.tape",
                super::import_statement().easy_parse("import packages.tape\n ").unwrap().0
            );
            // assert_eq!(
            //     TasklikeBlock {
            //         name: "task_name",
            //         subtype: BlockType::Task,
            //         specs: vec![BlockSpec::package("package_name")],
            //         code: BashCode {
            //             code: "echo 'hi'",
            //             vars: HashSet::default(),
            //         }
            //     },
            //     super::task().easy_parse(
            //         "task task_name\n  : package_name\n{\n  echo 'hi'\n}"
            //     ).unwrap().0
            // );
            Ok(())
        }
    }
}

mod tapefile {
    use super::{
        config::global_config,
        misc::{import_statement, module_statement},
        plan::plan,
        prelude::*,
        tasklike::task,
        util::lex,
    };
    use crate::ast::Item;

    p! {
        item() -> Item<'a>, {
            choice!(
                //versioner().map(Item::Versioner),
                import_statement().map(Item::Import),
                module_statement().map(|(k, v)| Item::Module(k, v)),
                task().map(Item::Task),
                global_config().map(Item::GlobalConfig),
                plan().map(Item::Plan)
                // NB this wouldn't parse, b/c the "p" gets picked up by "plan":
                // package().map(Item::Package)

            )
        }
    }

    p! {
        items() -> Vec<Item<'a>>, {
            many(lex(item()))
        }
    }
}
