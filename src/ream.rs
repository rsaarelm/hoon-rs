//! Parse hoon source into AST twig

use std::str;
use std::str::FromStr;
use nom::*;
use nom::IResult::*;
use nom::Err::*;

use twig::{Twig, Rune, Odor};

#[inline]
pub fn is_lowercase(chr: u8) -> bool {
    chr >= 'a' as u8 && chr <= 'z' as u8
}

/// Match at least two spaces or one newline.
pub fn long_space(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let mut spaces = 0;
    let mut idx = 0;
    for item in input.iter() {
        if *item == '\t' as u8 {
            // Physical tabs are banned.
            // XXX: Should have own ErrorKind tag.
            return Error(Position(ErrorKind::MultiSpace, input));
        }

        if *item == '\n' as u8 {
            spaces += 2;
        } else if *item == ' ' as u8 {
            spaces += 1;
        }

        if *item != ' ' as u8 && *item != '\r' as u8 && *item != '\n' as u8 {
            break;
        }

        idx += 1;
    }

    if spaces < 2 {
        Error(Position(ErrorKind::MultiSpace, input))
    } else {
        Done(&input[idx..], &input[0..idx])
    }
}

pub fn ident(input:&[u8]) -> IResult<&[u8], &[u8]> {
    for (idx, item) in input.iter().enumerate() {
        if idx == 0 {
            // TODO: Should we only accept lowercase chars?
            if !is_alphabetic(*item) {
                return Error(Position(ErrorKind::Alpha, input))
            }
        } else {
            if !is_alphabetic(*item) && !is_digit(*item) && *item != '-' as u8 {
                return Done(&input[idx..], &input[0..idx])
            }
        }
    }
    Error(Position(ErrorKind::Alpha, input))
}

named!(comment<&[u8]>,
    chain!(
        tag!("::") ~
        x: take_until_and_consume!("\n"),
        || { x }
    )
);

// A valid gap is any sequence of long spaces and comments.
named!(gap< Vec<&[u8]> >,
    many1!(
        alt!(
            long_space
          | comment
        )
    )
);

// TODO: Handle separator dots
// TODO: Handle other odors than ud.
named!(ud<Twig>,
  map_res!(
    map_res!(
      map_res!(
        digit,
        str::from_utf8
      ),
      FromStr::from_str
    ),
    |x| Ok::<Twig, ()>(Twig::Cell(
            box Twig::Rune(Rune::dtzy),
            box Twig::Atom(Odor::ud, x)))
  )
);

// TODO: Don't have two different-named identifier parsers that differ just on
// having the from_utf8...
/// Parse an identifier name.
named!(id<&str>,
    map_res!(
        ident,
        str::from_utf8
    )
);

/// Terminator for an arbitrary-length tall rune.
named!(tall_terminator<()>,
    chain!(
        gap ~
        tag!("=="),
      || ()
    )
);


macro_rules! tall_rune_args {
    ($i: expr, $first: tt) => {
        chain!($i,
            x: $first ~
            gap,
         || { x }
         )
    };
    ($i: expr, $first: tt, $($rest: tt),+) => {
        chain!($i,
            x: $first ~
            gap ~
            xs: tall_rune_args!($($rest),*),
         || { Twig::Cell(box x, box xs) }
         )
    };
}

macro_rules! wide_rune_args {
    ($i: expr, $first: tt) => {
        chain!($i,
            x: $first ~
            tag!(")"),
         || { x }
         )
    };
    ($i: expr, $first: tt, $($rest: tt),+) => {
        chain!($i,
            x: $first ~
            tag!(" ") ~
            xs: wide_rune_args!($($rest),*),
         || { Twig::Cell(box x, box xs) }
         )
    };
}

/// A standard rune that may have either a wide or a tall form.
macro_rules! rune {
    ($i: expr, $name:ident, $($parser: tt),+) => {
        chain!($i,
            tag!(Rune::$name.glyph()) ~
            args: alt!(
                chain!(
                    tag!("(") ~
                    args: wide_rune_args!($($parser),*),
                    || { args }
                ) |
                chain!(
                    gap ~
                    args: tall_rune_args!($($parser),*),
                    || { args }
                )
            ),
            || { Twig::Cell(box Twig::Rune(Rune::$name), box args) }
        )
    }
}

/// Parse a Hoon expression into an AST.
named!(pub ream<Twig>,
    alt!(
        rune!(brhp, ream) |
        ud
        /*
      | Brhp
      | Dtls
      | Dtts
      | Ktts
      | Tsgr
      | Tsls
      | Wtcl
      */

        // TODO: Rest of hoon
    )
);

/*
named!(p<(Box<Twig>)>,
    alt!(
        chain!(
            p: preceded!(gap, ream),
            || { Box::new(p) })
        | delimited!(
            tag!("("),
            chain!(
                p: ream,
                || { Box::new(p) }),
            tag!(")"))
    )
);

named!(pq<(Box<Twig>, Box<Twig>)>,
    alt!(
        chain!(
            p: preceded!(gap, ream) ~
            q: preceded!(gap, ream),
            || { (Box::new(p), Box::new(q)) })
        | delimited!(
            tag!("("),
            chain!(
                p: ream ~
                space ~
                q: ream,
                || { (Box::new(p), Box::new(q)) }),
            tag!(")"))
    )
);

named!(pqr<(Box<Twig>, Box<Twig>, Box<Twig>)>,
    alt!(
        chain!(
            p: preceded!(gap, ream) ~
            q: preceded!(gap, ream) ~
            r: preceded!(gap, ream),
            || { (Box::new(p), Box::new(q), Box::new(r)) })
        | delimited!(
            tag!("("),
            chain!(
                p: ream ~
                space ~
                q: ream ~
                space ~
                r: ream,
                || { (Box::new(p), Box::new(q), Box::new(r)) }),
            tag!(")"))
    )
);

/// Regular rune with 1 argument
macro_rules! rune1 {
    ($id:ident, $rune:expr) => {
        named!($id<Twig>,
           chain!(
               tag!($rune) ~
               p: p,
               || {
                 Twig::$id(p)
               }
            )
        );
    }
}

/// Regular rune with 2 arguments
macro_rules! rune2 {
    ($id:ident, $rune:expr) => {
        named!($id<Twig>,
           chain!(
               tag!($rune) ~
               a: pq,
               || {
                 let (p, q) = a.clone();
                 Twig::$id(p, q)
               }
            )
        );
    }
}

/// Regular rune with 3 arguments
macro_rules! rune3 {
    ($id:ident, $rune:expr) => {
        named!($id<Twig>,
           chain!(
               tag!($rune) ~
               a: pqr,
               || {
                 let (p, q, r) = a.clone();
                 Twig::$id(p, q, r)
               }
            )
        );
    }
}

named!(wing<Wing>,
    chain!(
        x: id ~
        mut xs: many0!(
            chain!(
                tag!(".") ~
                x: id,
                || { x.to_string() }
            )
        ),
        || {
            xs.insert(0, x.to_string());
            xs
        }
    )
);

rune1!(Brhp, "|-");
rune1!(Dtls, ".+");
rune2!(Dtts, ".=");
rune2!(Ktts, "^=");
rune2!(Tsgr, "=>");
rune2!(Tsls, "=+");
rune3!(Wtcl, "?:");
*/

/*
named!(Cnts<Twig>,
    // TODO: Tall form
    delimited!(
        tag!("%=("),
        chain!(
            p: wing ~
            space ~
            q1: wing ~
            space ~
            q2: ream ~
            rs: many0!(
                chain!(
                    tag!(",") ~
                    space ~
                    r1: wing ~
                    space ~
                    r2: ream,
                || { (r1, r2) }
                )
            ),

            || {
                rs.insert(0, (q1, q2));
                Twig::Cnts(p, rs)
            }),
        tag!(")"))
);
*/

#[cfg(test)]
mod test {
    use super::gap;

    #[test]
    fn test_parse_gap() {
        assert!(gap(&b"  "[..]).is_done());
        assert!(gap(&b"    "[..]).is_done());
        assert!(gap(&b"\n"[..]).is_done());
        assert!(gap(&b"\n  "[..]).is_done());
        assert!(gap(&b"  \n  "[..]).is_done());
        assert!(!gap(&b" "[..]).is_done());
    }
}
