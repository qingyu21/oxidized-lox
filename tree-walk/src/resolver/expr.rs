use super::{ClassType, ResolveError, Resolver};
use crate::expr::{Expr, ExprArena};

impl<'a> Resolver<'a> {
    // Public entry point for resolving a standalone expression, mainly for the REPL.
    pub(crate) fn resolve_expression(
        &mut self,
        expression: &Expr,
        expr_arena: &ExprArena,
    ) -> Result<(), ResolveError> {
        self.resolve_expression_node(expression, expr_arena)
    }

    // Recursively walk one expression node and resolve any variable reads or
    // writes it contains to their lexical scope distance.
    pub(super) fn resolve_expression_node(
        &mut self,
        expression: &Expr,
        expr_arena: &ExprArena,
    ) -> Result<(), ResolveError> {
        match expression {
            Expr::Assign { name, value } => {
                self.resolve_expression_node(expr_arena.get(*value), expr_arena)?;
                self.resolve_local(name, false);
                Ok(())
            }
            Expr::Binary { left, right, .. } => {
                self.resolve_expression_node(expr_arena.get(*left), expr_arena)?;
                self.resolve_expression_node(expr_arena.get(*right), expr_arena)
            }
            Expr::Call {
                callee, arguments, ..
            } => {
                self.resolve_expression_node(expr_arena.get(*callee), expr_arena)?;
                for argument in arguments {
                    self.resolve_expression_node(expr_arena.get(*argument), expr_arena)?;
                }
                Ok(())
            }
            Expr::Get { object, .. } => {
                self.resolve_expression_node(expr_arena.get(*object), expr_arena)
            }
            Expr::Grouping { expression } => {
                self.resolve_expression_node(expr_arena.get(*expression), expr_arena)
            }
            Expr::Literal { .. } => Ok(()),
            Expr::Logical { left, right, .. } => {
                self.resolve_expression_node(expr_arena.get(*left), expr_arena)?;
                self.resolve_expression_node(expr_arena.get(*right), expr_arena)
            }
            Expr::Set { object, value, .. } => {
                self.resolve_expression_node(expr_arena.get(*value), expr_arena)?;
                self.resolve_expression_node(expr_arena.get(*object), expr_arena)
            }
            Expr::Super { keyword, .. } => {
                if self.current_class == ClassType::None {
                    return Err(self.error(keyword, "Can't use 'super' outside of a class."));
                }
                if self.current_class != ClassType::Subclass {
                    return Err(
                        self.error(keyword, "Can't use 'super' in a class with no superclass.")
                    );
                }

                self.resolve_local(keyword, true);
                Ok(())
            }
            Expr::This { keyword } => {
                if self.current_class == ClassType::None {
                    return Err(self.error(keyword, "Can't use 'this' outside of a class."));
                }

                self.resolve_local(keyword, true);
                Ok(())
            }
            Expr::Variable { name } => {
                if self
                    .scopes
                    .last()
                    .and_then(|scope| scope.bindings.get(name.lexeme.as_ref()))
                    .is_some_and(|binding| !binding.defined)
                {
                    return Err(
                        self.error(name, "Can't read local variable in its own initializer.")
                    );
                }

                self.resolve_local(name, true);
                Ok(())
            }
            Expr::Conditional {
                condition,
                then_branch,
                else_branch,
            } => {
                self.resolve_expression_node(expr_arena.get(*condition), expr_arena)?;
                self.resolve_expression_node(expr_arena.get(*then_branch), expr_arena)?;
                self.resolve_expression_node(expr_arena.get(*else_branch), expr_arena)
            }
            Expr::Unary { right, .. } => {
                self.resolve_expression_node(expr_arena.get(*right), expr_arena)
            }
        }
    }
}
