use crate::parser::{directives::{DirectiveState, Directives}, lexer::{LexDirective, Lexer, PrecToken}};


pub fn generate_partial_lex<'src>(source: &'src str, directives: &Directives) -> Vec<PrecToken<'src>> {
    let lexer = Lexer::new(source);
    let mut tree = lexer.split_directives();
    let mut directives = directives.clone();
    expand_directives_simple(&mut tree, &mut directives, true, true);

    return vec![];
}

fn expand_directives_simple(tree: &mut Vec<PrecToken>, directives: &mut Directives, mut currently_active: bool, parent_active: bool) {
    for element in tree {
        if let PrecToken::Directive(lex_directive) = element {
            match lex_directive {
                LexDirective::Define(name) => if currently_active { directives.define(name) },
                LexDirective::Undef(name) => if currently_active { directives.undef(name) },
                LexDirective::IfDef { directive, state, body } => {
                    let parent_active = currently_active;
                    currently_active = parent_active && directives.is_defined(directive);
                    *state = DirectiveState::Evaluated(currently_active);
                    expand_directives_simple(body, directives, currently_active, parent_active);
                },
                LexDirective::IfNDef { directive, state, body } => {
                    let parent_active = currently_active;
                    currently_active = parent_active && !directives.is_defined(directive);
                    *state = DirectiveState::Evaluated(currently_active);
                    expand_directives_simple(body, directives, currently_active, parent_active);
                },
                LexDirective::IfOpt { state, body, .. } => {
                    // IFOPT is used so rarely, that I will defer the actual implementation of this until
                    // I really think it will be necessery.
                    *state = DirectiveState::Evaluated(currently_active);
                    expand_directives_simple(body, directives, currently_active, parent_active);
                },
                LexDirective::Else { body } => {
                    currently_active = parent_active && !currently_active;
                    expand_directives_simple(body, directives, currently_active, parent_active);
                }
                _ => {}
            }
        }
    }
}