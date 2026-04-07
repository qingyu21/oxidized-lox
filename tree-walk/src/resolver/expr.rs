use super::{ClassType, ResolveError, Resolver};
use crate::expr::Expr;

impl<'a> Resolver<'a> {
    // Public entry point for resolving a standalone expression, mainly for the REPL.
    pub(crate) fn resolve_expression(&mut self, expression: &Expr) -> Result<(), ResolveError> {
        self.resolve_expression_node(expression)
    }

    // Recursively walk one expression node and resolve any variable reads or
    // writes it contains to their lexical scope distance.
    pub(super) fn resolve_expression_node(
        &mut self,
        expression: &Expr,
    ) -> Result<(), ResolveError> {
        match expression {
            Expr::Assign { name, value } => {
                self.resolve_expression_node(value)?;
                self.resolve_local(name, false);
                Ok(())
            }
            Expr::Binary { left, right, .. } => {
                self.resolve_expression_node(left)?;
                self.resolve_expression_node(right)
            }
            Expr::Call {
                callee, arguments, ..
            } => {
                self.resolve_expression_node(callee)?;
                for argument in arguments {
                    self.resolve_expression_node(argument)?;
                }
                Ok(())
            }
            Expr::Get { object, .. } => self.resolve_expression_node(object),
            Expr::Grouping { expression } => self.resolve_expression_node(expression),
            Expr::Literal { .. } => Ok(()),
            Expr::Logical { left, right, .. } => {
                self.resolve_expression_node(left)?;
                self.resolve_expression_node(right)
            }
            Expr::Set { object, value, .. } => {
                self.resolve_expression_node(value)?;
                self.resolve_expression_node(object)
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
                    .and_then(|scope| scope.get(&name.lexeme))
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
                self.resolve_expression_node(condition)?;
                self.resolve_expression_node(then_branch)?;
                self.resolve_expression_node(else_branch)
            }
            Expr::Unary { right, .. } => self.resolve_expression_node(right),
        }
    }
}
