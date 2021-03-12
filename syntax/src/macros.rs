macro_rules! p (
    ($name:ident( $($arg: ident :  $arg_type: ty),* ) -> $ret:ty, $code:expr) => (
        combine::parser!{
            pub fn $name['a, I]($($arg : $arg_type),*)(I) -> $ret
                where
                [I: combine::stream::RangeStream<
                 Range = &'a str,
                 Token = char>,
                 I::Error: combine::ParseError<char, &'a str, <I as combine::stream::StreamOnce>::Position>,
                 <I::Error as combine::ParseError<char, &'a str, <I as combine::stream::StreamOnce>::Position>>::StreamError:
                 From<std::num::ParseIntError> +
                 From<std::num::ParseFloatError>
            ]            {
                $code
            }
        }
    );
);

macro_rules! wrapper {
    ($name:ident($delegate: ident), $code:expr) => (
        combine::parser!{
            pub fn $name['a, I, P]($delegate: P)(I) -> P::Output
                where
                [I: combine::stream::RangeStream<
                 Range = &'a str,
                 Token = char>,
                 I::Error: combine::ParseError<char, &'a str, <I as combine::stream::StreamOnce>::Position>,
                 <I::Error as combine::ParseError<char, &'a str, <I as combine::stream::StreamOnce>::Position>>::StreamError:
                 From<std::num::ParseIntError> +
                 From<std::num::ParseFloatError>,
                 P: combine::Parser<I>,
            ]            {
                $code
            }
        }
    );
}

macro_rules! repeater {
    ($name:ident($delegate: ident), $code:expr) => (
        combine::parser!{
            pub fn $name['a, I, P]($delegate: P)(I) -> Vec<P::Output>
                where
                [I: combine::stream::RangeStream<
                 Range = &'a str,
                 Token = char>,
                 I::Error: combine::ParseError<char, &'a str, <I as combine::stream::StreamOnce>::Position>,
                 <I::Error as combine::ParseError<char, &'a str, <I as combine::stream::StreamOnce>::Position>>::StreamError:
                 From<std::num::ParseIntError> +
                 From<std::num::ParseFloatError>,
                 P: combine::Parser<I>,
            ]            {
                $code
            }
        }
    );
}
