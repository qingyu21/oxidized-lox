use std::{fmt, ops::Deref, rc::Rc};

use crate::token::{Literal, Token};

pub(crate) type ExprArenaRef = Rc<ExprArena>;

#[allow(clippy::vec_box)]
#[derive(Debug, Default)]
pub(crate) struct ExprArena {
    // Box each node so arena growth never relocates previously stored
    // expressions; child ExprRefs rely on those stable addresses.
    nodes: Vec<Box<Expr>>,
}

#[derive(Clone, Copy)]
pub(crate) struct ExprRef(*const Expr);

#[derive(Debug, Clone)]
pub(crate) enum Expr {
    Assign {
        name: Token,
        value: ExprRef,
    },
    Binary {
        left: ExprRef,
        operator: Token,
        right: ExprRef,
    },
    Call {
        callee: ExprRef,
        paren: Token,
        arguments: Vec<ExprRef>,
    },
    Get {
        object: ExprRef,
        name: Token,
    },
    Grouping {
        expression: ExprRef,
    },
    Literal {
        value: Literal,
    },
    Logical {
        left: ExprRef,
        operator: Token,
        right: ExprRef,
    },
    Set {
        object: ExprRef,
        name: Token,
        value: ExprRef,
    },
    Super {
        keyword: Token,
        method: Token,
    },
    This {
        keyword: Token,
    },
    Variable {
        name: Token,
    },
    Conditional {
        condition: ExprRef,
        then_branch: ExprRef,
        else_branch: ExprRef,
    },
    Unary {
        operator: Token,
        right: ExprRef,
    },
}

impl Expr {
    // Construct an assignment expression that updates an existing binding.
    pub(crate) fn assign(exprs: &mut ExprArena, name: Token, value: Expr) -> Self {
        Expr::Assign {
            name,
            value: exprs.alloc(value),
        }
    }

    // Construct a binary expression with left and right operands.
    pub(crate) fn binary(exprs: &mut ExprArena, left: Expr, operator: Token, right: Expr) -> Self {
        Expr::Binary {
            left: exprs.alloc(left),
            operator,
            right: exprs.alloc(right),
        }
    }

    // Construct a function call expression with a callee and zero or more arguments.
    pub(crate) fn call(
        exprs: &mut ExprArena,
        callee: Expr,
        paren: Token,
        arguments: Vec<Expr>,
    ) -> Self {
        Expr::Call {
            callee: exprs.alloc(callee),
            paren,
            arguments: arguments
                .into_iter()
                .map(|argument| exprs.alloc(argument))
                .collect(),
        }
    }

    // Construct a property read expression like `object.name`.
    pub(crate) fn get(exprs: &mut ExprArena, object: Expr, name: Token) -> Self {
        Expr::Get {
            object: exprs.alloc(object),
            name,
        }
    }

    // Construct a grouping expression that preserves explicit parentheses.
    pub(crate) fn grouping(exprs: &mut ExprArena, expression: Expr) -> Self {
        Expr::Grouping {
            expression: exprs.alloc(expression),
        }
    }

    // Construct a literal expression from an already-parsed literal value.
    pub(crate) fn literal(value: Literal) -> Self {
        Expr::Literal { value }
    }

    // Construct a logical expression that may short-circuit.
    pub(crate) fn logical(exprs: &mut ExprArena, left: Expr, operator: Token, right: Expr) -> Self {
        Expr::Logical {
            left: exprs.alloc(left),
            operator,
            right: exprs.alloc(right),
        }
    }

    // Construct a property assignment like `object.name = value`.
    pub(crate) fn set(exprs: &mut ExprArena, object: ExprRef, name: Token, value: Expr) -> Self {
        Expr::Set {
            object,
            name,
            value: exprs.alloc(value),
        }
    }

    // Construct a `super.method` expression used inside subclasses.
    pub(crate) fn super_(keyword: Token, method: Token) -> Self {
        Expr::Super { keyword, method }
    }

    // Construct a `this` expression used inside methods.
    pub(crate) fn this(keyword: Token) -> Self {
        Expr::This { keyword }
    }

    // Construct a variable expression that refers to a named binding.
    pub(crate) fn variable(name: Token) -> Self {
        Expr::Variable { name }
    }

    // Construct a conditional expression with then/else branches.
    pub(crate) fn conditional(
        exprs: &mut ExprArena,
        condition: Expr,
        then_branch: Expr,
        else_branch: Expr,
    ) -> Self {
        Expr::Conditional {
            condition: exprs.alloc(condition),
            then_branch: exprs.alloc(then_branch),
            else_branch: exprs.alloc(else_branch),
        }
    }

    // Construct a unary expression with one operand.
    pub(crate) fn unary(exprs: &mut ExprArena, operator: Token, right: Expr) -> Self {
        Expr::Unary {
            operator,
            right: exprs.alloc(right),
        }
    }
}

impl ExprArena {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn alloc(&mut self, expr: Expr) -> ExprRef {
        self.nodes.push(Box::new(expr));
        ExprRef(
            self.nodes
                .last()
                .expect("arena should contain the expression that was just inserted")
                .as_ref() as *const Expr,
        )
    }

    pub(crate) fn into_shared(self) -> ExprArenaRef {
        Rc::new(self)
    }
}

impl ExprRef {
    fn as_expr(&self) -> &Expr {
        // SAFETY: `ExprRef` points at an `Expr` owned by a boxed node inside an
        // `ExprArena`. The arena stores each node at a stable address, and all
        // parser outputs carrying these refs keep the arena alive for at least
        // as long as the expression tree is traversed.
        unsafe { &*self.0 }
    }
}

impl Deref for ExprRef {
    type Target = Expr;

    fn deref(&self) -> &Self::Target {
        self.as_expr()
    }
}

impl AsRef<Expr> for ExprRef {
    fn as_ref(&self) -> &Expr {
        self.as_expr()
    }
}

impl fmt::Debug for ExprRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_expr().fmt(f)
    }
}
