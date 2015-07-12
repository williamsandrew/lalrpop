/*!
 * Compiled representation of a grammar. Simplified, normalized
 * version of `parse_tree`. The normalization passes produce this
 * representation incrementally.
 */

use intern::{InternedString};
use grammar::pattern::{Pattern, PatternKind};
use std::iter::once;
use std::fmt::{Debug, Display, Formatter, Error};
use util::{map, Map, Sep};

// These concepts we re-use wholesale
pub use grammar::parse_tree::{NonterminalString,
                              Span,
                              TerminalString, TypeParameter};

#[derive(Clone, Debug)]
pub struct Grammar {
    // a unique prefix that can be appended to identifiers to ensure
    // that they do not conflict with any action strings
    pub prefix: String,

    // these are the nonterminals that were declared to be public; the
    // key is the user's name for the symbol, the value is the
    // artificial symbol we introduce, which will always have a single
    // production like `Foo' = Foo`.
    pub start_nonterminals: Map<NonterminalString, NonterminalString>,

    // the "use foo;" statements that the user declared
    pub uses: Vec<String>,
    pub type_parameters: Vec<TypeParameter>,
    pub parameters: Vec<Parameter>,
    pub where_clauses: Vec<String>,

    // the grammar proper:

    pub action_fn_defns: Vec<ActionFnDefn>,
    pub productions: Map<NonterminalString, Vec<Production>>,
    pub token_span: Span,
    pub conversions: Map<TerminalString, Pattern<TypeRepr>>,
    pub types: Types,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parameter {
    pub name: InternedString,
    pub ty: TypeRepr,
}

#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Production {
    // this overlaps with the key in the hashmap, obviously, but it's
    // handy to have it
    pub nonterminal: NonterminalString,
    pub symbols: Vec<Symbol>,
    pub action_fn: ActionFn,
    pub span: Span,
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Symbol {
    Nonterminal(NonterminalString),
    Terminal(TerminalString),
}

#[derive(Clone, PartialEq, Eq)]
pub struct ActionFnDefn {
    pub arg_patterns: Vec<InternedString>,
    pub arg_types: Vec<TypeRepr>,
    pub ret_type: TypeRepr,
    pub code: String,
}

#[derive(Clone, PartialEq, Eq)]
pub enum TypeRepr {
    Tuple(Vec<TypeRepr>),
    Nominal(NominalTypeRepr),
    Lifetime(InternedString),
}

#[derive(Clone, PartialEq, Eq)]
pub struct NominalTypeRepr {
    pub path: Vec<InternedString>,
    pub types: Vec<TypeRepr>
}

#[derive(Clone, Debug)]
pub struct Types {
    terminal_enum_type: NominalTypeRepr,
    default_terminal_type: TypeRepr,
    terminal_types: Map<TerminalString, TypeRepr>,
    nonterminal_types: Map<NonterminalString, TypeRepr>
}

impl Types {
    pub fn new(terminal_enum_type: NominalTypeRepr) -> Types {
        Types { terminal_enum_type: terminal_enum_type.clone(),
                terminal_types: map(),
                default_terminal_type: TypeRepr::Nominal(terminal_enum_type),
                nonterminal_types: map() }
    }

    pub fn add_type(&mut self, nt_id: NonterminalString, ty: TypeRepr) {
        assert!(self.nonterminal_types.insert(nt_id, ty).is_none());
    }

    pub fn add_term_type(&mut self, term: TerminalString, ty: TypeRepr) {
        assert!(self.terminal_types.insert(term, ty).is_none());
    }

    pub fn terminal_enum_type(&self) -> &NominalTypeRepr {
        &self.terminal_enum_type
    }

    pub fn terminal_type(&self, id: TerminalString) -> &TypeRepr {
        self.terminal_types.get(&id).unwrap_or(&self.default_terminal_type)
    }

    pub fn lookup_nonterminal_type(&self, id: NonterminalString) -> Option<&TypeRepr> {
        self.nonterminal_types.get(&id)
    }

    pub fn nonterminal_type(&self, id: NonterminalString) -> &TypeRepr {
        &self.nonterminal_types[&id]
    }
}

impl Display for Parameter {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        write!(fmt, "{}: {}", self.name, self.ty)
    }
}

impl Display for TypeRepr {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        match *self {
            TypeRepr::Tuple(ref types) =>
                write!(fmt, "({})", Sep(", ", types)),
            TypeRepr::Nominal(ref data) =>
                write!(fmt, "{}", data),
            TypeRepr::Lifetime(id) =>
                write!(fmt, "{}", id),
        }
    }
}

impl Debug for TypeRepr {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        Display::fmt(self, fmt)
    }
}

impl Display for NominalTypeRepr {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        if self.types.len() == 0 {
            write!(fmt, "{}", Sep("::", &self.path))
        } else {
            write!(fmt, "{}<{}>", Sep("::", &self.path), Sep(", ", &self.types))
        }
    }
}

impl Debug for NominalTypeRepr {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        Display::fmt(self, fmt)
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct ActionFn(u32);

impl ActionFn {
    pub fn new(x: usize) -> ActionFn {
        ActionFn(x as u32)
    }

    pub fn index(&self) -> usize {
        self.0 as usize
    }
}

impl Symbol {
    pub fn ty<'ty>(&self, t: &'ty Types) -> &'ty TypeRepr {
        match *self {
            Symbol::Terminal(id) => t.terminal_type(id),
            Symbol::Nonterminal(id) => t.nonterminal_type(id),
        }
    }
}

impl Display for Symbol {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        match *self {
            Symbol::Nonterminal(id) => write!(fmt, "{}", id),
            Symbol::Terminal(id) => write!(fmt, "{}", id),
        }
    }
}

impl Debug for Symbol {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        Display::fmt(self, fmt)
    }
}

impl Debug for Production {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        write!(fmt,
               "{} = {} => {:?};",
               self.nonterminal, Sep(", ", &self.symbols), self.action_fn)
    }
}

impl Debug for ActionFnDefn {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        write!(fmt, "{}", self.to_fn_string("_"))
    }
}

impl ActionFnDefn {
    fn to_fn_string(&self, name: &str) -> String {
        let arg_strings: Vec<String> =
               self.arg_patterns
                   .iter()
                   .zip(self.arg_types.iter())
                   .map(|(p, t)| format!("{}: {}", p, t))
                   .collect();

        format!("fn {}({}) -> {} {{ {} }}",
                name, Sep(", ", &arg_strings), self.ret_type, self.code)
    }
}

impl Grammar {
    pub fn default_pattern(&self, id: InternedString) -> Pattern<TypeRepr> {
        let path: Vec<InternedString> =
            self.types.terminal_enum_type().path.iter()
                                                .cloned()
                                                .chain(once(id))
                                                .collect();
        Pattern {
            span: self.token_span,
            kind: PatternKind::Enum(path, vec![
                Pattern {
                    span: self.token_span,
                    kind: PatternKind::DotDot
                }
            ])
        }
    }

    pub fn pattern(&self, t: TerminalString) -> Pattern<TypeRepr> {
        match self.conversions.get(&t).cloned() {
            Some(p) => p,
            None => self.default_pattern(t.0),
        }
    }

    pub fn productions_for(&self, nonterminal: NonterminalString) -> &[Production] {
        match self.productions.get(&nonterminal) {
            Some(v) => &v[..],
            None => &[], // this...probably shouldn't happen actually?
        }
    }
}
