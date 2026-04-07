use std::collections::HashMap;

use super::{BindingInfo, BindingKind, ResolveError, Resolver};
use crate::{
    interpreter::ResolvedBinding,
    lox,
    token::{Token, TokenType},
};

impl<'a> Resolver<'a> {
    pub(super) fn define_this(&mut self, line: u32) {
        let Some(scope) = self.scopes.last_mut() else {
            return;
        };

        let token = Token::new(TokenType::This, "this".to_string(), None, line);
        scope.insert(
            "this".to_string(),
            BindingInfo {
                token,
                kind: BindingKind::This,
                defined: true,
                used: false,
            },
        );
    }

    pub(super) fn define_super(&mut self, line: u32) {
        let Some(scope) = self.scopes.last_mut() else {
            return;
        };

        let token = Token::new(TokenType::Super, "super".to_string(), None, line);
        scope.insert(
            "super".to_string(),
            BindingInfo {
                token,
                kind: BindingKind::Super,
                defined: true,
                used: false,
            },
        );
    }

    // Push a fresh lexical scope for a block or function body.
    pub(super) fn begin_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    // Pop the innermost lexical scope once resolution leaves it, reporting a
    // resolver error if a local variable in that scope was never read.
    pub(super) fn end_scope(&mut self) -> Result<(), ResolveError> {
        let Some(scope) = self.scopes.pop() else {
            return Ok(());
        };

        let unused = scope
            .values()
            .filter(|binding| {
                binding.kind == BindingKind::Variable && binding.defined && !binding.used
            })
            .min_by_key(|binding| binding.token.id);

        if let Some(binding) = unused {
            return Err(self.error(
                &binding.token,
                &format!("Local variable '{}' is never used.", binding.token.lexeme),
            ));
        }

        Ok(())
    }

    pub(super) fn discard_scope(&mut self) {
        self.scopes.pop();
    }

    pub(super) fn finish_scope(
        &mut self,
        result: Result<(), ResolveError>,
    ) -> Result<(), ResolveError> {
        match result {
            Ok(()) => self.end_scope(),
            Err(error) => {
                self.discard_scope();
                Err(error)
            }
        }
    }

    // Record a name in the current scope before its initializer resolves so
    // reads from the variable's own initializer can be rejected.
    pub(super) fn declare(&mut self, name: &Token, kind: BindingKind) -> Result<(), ResolveError> {
        let Some(scope) = self.scopes.last_mut() else {
            return Ok(());
        };

        if scope.contains_key(&name.lexeme) {
            return Err(self.error(name, "Already a variable with this name in this scope."));
        }

        scope.insert(
            name.lexeme.clone(),
            BindingInfo {
                token: name.clone(),
                kind,
                defined: false,
                used: false,
            },
        );
        Ok(())
    }

    // Mark a previously declared local as fully available for reads.
    pub(super) fn define(&mut self, name: &Token) {
        if let Some(binding) = self
            .scopes
            .last_mut()
            .and_then(|scope| scope.get_mut(&name.lexeme))
        {
            binding.defined = true;
        }
    }

    // Find how many scopes outward this name resolves to and hand that lexical
    // distance to the interpreter for later fast runtime lookup.
    pub(super) fn resolve_local(&mut self, name: &Token, mark_used: bool) {
        for (distance, scope) in self.scopes.iter_mut().rev().enumerate() {
            if let Some(binding) = scope.get_mut(&name.lexeme) {
                if mark_used {
                    binding.used = true;
                }
                self.interpreter
                    .resolve(name, ResolvedBinding::Local(distance));
                return;
            }
        }

        self.interpreter.resolve(name, ResolvedBinding::Global);
    }

    // Report a resolver error through the shared Lox error reporter and stop
    // the current resolution walk.
    pub(super) fn error(&self, token: &Token, message: &str) -> ResolveError {
        lox::token_error(token, message);
        ResolveError
    }
}
