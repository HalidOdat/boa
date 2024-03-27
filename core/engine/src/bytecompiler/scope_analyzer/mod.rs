use std::ops::ControlFlow;

use boa_ast::{
    declaration::LexicalDeclaration,
    expression::{Identifier, IdentifierScope},
    visitor::{VisitWith, VisitorMut},
    Script,
};
use boa_interner::Sym;
use rustc_hash::FxHashMap;

#[cfg(test)]
mod tests;

/// Returns `true` if the given statement can optimize local variables.
#[must_use]
pub fn scope_analyzer(node: &mut Script, strict: bool) -> bool {
    let mut visitor = ScopeAnalyzer::new(strict);
    visitor.visit(node).is_continue()
}

#[derive(Debug)]
struct EnvironmentScope {
    environment_index: u32,
    parent_environment_index: Option<u32>,
    map: FxHashMap<Sym, u32>,
}

impl EnvironmentScope {
    fn insert_sym(&mut self, sym: Sym) -> u32 {
        let index = self.map.len() as u32;
        println!("    INSERT SYM: {}, index: {index}", sym.get());
        let previous = self.map.insert(sym, index);
        assert!(previous.is_none());
        index
    }

    // fn remove_sym(&mut self, sym: Sym) {
    //     let previous = self.map.remove(&sym);
    //     assert!(previous.is_some());
    // }
}

#[derive(Debug)]
struct FunctionScope {
    strict: bool,
    function_index: u32,
    parent_function_index: Option<u32>,

    environment_index: u32,
    pub(crate) scopes: Vec<EnvironmentScope>,
    // max_symbol_index: u32,
}

impl FunctionScope {
    fn global(strict: bool) -> Self {
        Self {
            function_index: 0,
            parent_function_index: None,
            environment_index: 0,
            strict,
            // max_symbol_index: 0,
            scopes: vec![EnvironmentScope {
                environment_index: 0,
                map: FxHashMap::default(),
                parent_environment_index: None,
            }],
        }
    }

    fn current_scope(&mut self) -> &mut EnvironmentScope {
        &mut self.scopes[self.environment_index as usize]
    }
    fn push_scope(&mut self) -> u32 {
        println!("  PUSH SCOPE");
        let index = self.scopes.len() as u32;
        let current = self.current_scope();
        let function_scope = EnvironmentScope {
            environment_index: index,
            parent_environment_index: Some(current.environment_index),
            map: FxHashMap::default(),
        };
        self.scopes.push(function_scope);
        self.environment_index = index;
        index
    }
    fn pop_scope(&mut self) {
        println!("  POP SCOPE");
        let index = self.current_scope().parent_environment_index;
        self.function_index = index.unwrap_or_default();
    }

    fn get_sym(&self, sym: Sym) -> Option<u32> {
        for scope in &self.scopes {
            if let Some(index) = scope.map.get(&sym) {
                return Some(*index);
            }
        }
        None
    }

    fn insert_sym(&mut self, sym: Sym) -> u32 {
        self.current_scope().insert_sym(sym)
    }

    // fn remove_sym(&mut self, sym: Sym) {
    //     println!("    REMOVE SYM: {}", sym.get());
    //     self.current_scope().remove_sym(sym);
    // }
}

/// The [`Visitor`] used for [`returns_value`].
#[derive(Debug)]
pub(crate) struct ScopeAnalyzer {
    function_index: u32,
    functions: Vec<FunctionScope>,
}

impl ScopeAnalyzer {
    fn new(strict: bool) -> Self {
        Self {
            function_index: 0,
            functions: vec![FunctionScope::global(strict)],
        }
    }
    fn current_function(&mut self) -> &mut FunctionScope {
        self.functions
            .get_mut(self.function_index as usize)
            .expect("there should be a function scope")
    }
    fn push_function(&mut self, strict: bool) -> u32 {
        println!("PUSH FUNCTION");
        println!("  PUSH SCOPE");
        let index = self.functions.len() as u32;
        let current = self.current_function();
        let function_scope = FunctionScope {
            function_index: index,
            parent_function_index: Some(current.function_index),
            strict: current.strict || strict,
            // max_symbol_index: 0,
            scopes: vec![EnvironmentScope {
                environment_index: 0,
                map: FxHashMap::default(),
                parent_environment_index: None,
            }],
            environment_index: 0,
        };
        self.functions.push(function_scope);
        self.function_index = index;
        index
    }
    fn pop_function(&mut self) {
        println!("  POP SCOPE");
        println!("POP FUNCTION");
        let index = self.current_function().parent_function_index;
        self.function_index = index.unwrap_or_default();
    }
    fn push_scope(&mut self) -> u32 {
        self.current_function().push_scope()
    }
    fn pop_scope(&mut self) {
        self.current_function().pop_scope();
    }

    fn get_sym(&self, sym: Sym) -> Option<u32> {
        let mut function_index = Some(self.function_index);
        while let Some(index) = function_index {
            let function = &self.functions[index as usize];
            if let Some(index) = function.get_sym(sym) {
                return Some(index);
            }
            function_index = function.parent_function_index;
        }
        None
    }
}

impl<'ast> VisitorMut<'ast> for ScopeAnalyzer {
    type BreakTy = ();

    fn visit_identifier_mut(&mut self, node: &'ast mut Identifier) -> ControlFlow<Self::BreakTy> {
        if node.scope.is_some() {
            return ControlFlow::Continue(());
        }

        node.scope = Some(
            self.get_sym(node.sym())
                .map_or(IdentifierScope::Dynamic, IdentifierScope::Index),
        );

        ControlFlow::Continue(())
    }

    fn visit_block_mut(
        &mut self,
        node: &'ast mut boa_ast::statement::Block,
    ) -> ControlFlow<Self::BreakTy> {
        // TODO: Check if scope is need based on symbols in it.
        let index = self.push_scope();
        node.statement_list_mut().visit_with_mut(self)?;
        self.pop_scope();

        node.scope = Some(index);

        ControlFlow::Continue(())
    }

    fn visit_lexical_declaration_mut(
        &mut self,
        node: &'ast mut LexicalDeclaration,
    ) -> ControlFlow<Self::BreakTy> {
        match node {
            LexicalDeclaration::Const(list) => {
                for variable in list.as_mut() {
                    let ident = match variable.binding_mut() {
                        boa_ast::declaration::Binding::Identifier(ident) => ident,
                        boa_ast::declaration::Binding::Pattern(_) => return ControlFlow::Break(()),
                    };
                    let index = self.current_function().insert_sym(ident.sym());
                    assert!(ident.scope.is_none());
                    ident.scope = Some(IdentifierScope::Index(index));

                    let init = variable
                        .init_mut()
                        .expect("const always has init expression");
                    init.visit_with_mut(self)?;
                }
            }
            LexicalDeclaration::Let(list) => {
                for variable in list.as_mut() {
                    let ident = match variable.binding_mut() {
                        boa_ast::declaration::Binding::Identifier(ident) => ident,
                        boa_ast::declaration::Binding::Pattern(_) => return ControlFlow::Break(()),
                    };
                    let index = self.current_function().insert_sym(ident.sym());
                    assert!(ident.scope.is_none());
                    ident.scope = Some(IdentifierScope::Index(index));

                    if let Some(init) = variable.init_mut() {
                        init.visit_with_mut(self)?;
                    }
                }
            }
        }
        ControlFlow::Continue(())
    }

    fn visit_function_mut(
        &mut self,
        node: &'ast mut boa_ast::function::Function,
    ) -> ControlFlow<Self::BreakTy> {
        let function_index = self.push_function(node.body().strict());
        node.scope = Some(function_index);

        if let Some(ident) = &mut node.name_mut() {
            let binding_index = self
                .current_function()
                .current_scope()
                .insert_sym(ident.sym());

            let index = self.current_function().push_scope();

            ident.scope = Some(IdentifierScope::Index(binding_index));
            node.binding_scope = Some(index);
        }
        node.parameters_mut().visit_with_mut(self)?;
        node.body_mut().visit_with_mut(self)?;
        if node.name().is_some() {
            self.current_function().pop_scope();
        }

        self.pop_function();

        ControlFlow::Continue(())
    }

    fn visit_arrow_function_mut(
        &mut self,
        _node: &'ast mut boa_ast::function::ArrowFunction,
    ) -> ControlFlow<Self::BreakTy> {
        ControlFlow::Break(())
    }

    fn visit_async_function_mut(
        &mut self,
        node: &'ast mut boa_ast::function::AsyncFunction,
    ) -> ControlFlow<Self::BreakTy> {
        let function_index = self.push_function(node.body().strict());
        node.scope = Some(function_index);

        if let Some(ident) = &mut node.name_mut() {
            let binding_index = self
                .current_function()
                .current_scope()
                .insert_sym(ident.sym());

            let index = self.current_function().push_scope();

            ident.scope = Some(IdentifierScope::Index(binding_index));
            node.binding_scope = Some(index);
        }
        node.parameters_mut().visit_with_mut(self)?;
        node.body_mut().visit_with_mut(self)?;
        if node.name().is_some() {
            self.current_function().pop_scope();
        }

        self.pop_function();

        ControlFlow::Continue(())
    }

    fn visit_async_arrow_function_mut(
        &mut self,
        _node: &'ast mut boa_ast::function::AsyncArrowFunction,
    ) -> ControlFlow<Self::BreakTy> {
        ControlFlow::Break(())
        // todo!()
    }
}

// /// Returns `true` if the given statement can optimize local variables.
// #[must_use]
// pub fn can_optimize_local_variables<'a, N>(node: &'a mut N, strict: bool) -> bool
// where
//     &'a mut N: Into<NodeRefMut<'a>>,
// {
//     let mut visitor = CanOptimizeLocalVariables::new(strict);
//     let can_optimize_locals = visitor.visit(node.into()).is_continue();

//     can_optimize_locals
// }

// /// The [`Visitor`] used for [`returns_value`].
// #[derive(Debug)]
// struct CanOptimizeLocalVariables {
//     strict: bool,
//     uses_arguments: bool,
// }

// impl CanOptimizeLocalVariables {
//     const fn new(strict: bool) -> Self {
//         Self {
//             strict,
//             uses_arguments: false,
//         }
//     }
// }

// impl<'ast> VisitorMut<'ast> for CanOptimizeLocalVariables {
//     type BreakTy = ();

//     // fn visit_identifier_mut(&mut self, node: &'ast mut Identifier) -> ControlFlow<Self::BreakTy> {
//     //     if node.sym() == Sym::ARGUMENTS {
//     //         self.uses_arguments = true;
//     //     }

//     //     ControlFlow::Continue(())
//     // }

//     // fn visit_with_mut(
//     //     &mut self,
//     //     _node: &'ast mut boa_ast::statement::With,
//     // ) -> ControlFlow<Self::BreakTy> {
//     //     ControlFlow::Break(())
//     // }

//     // fn visit_call_mut(
//     //     &mut self,
//     //     node: &'ast mut boa_ast::expression::Call,
//     // ) -> ControlFlow<Self::BreakTy> {
//     //     if let Expression::Identifier(identifier) = node.function() {
//     //         if identifier.sym() == Sym::EVAL {
//     //             // Most likely a direct eval.
//     //             return ControlFlow::Break(());
//     //         }
//     //     }

//     //     try_break!(node.function_mut().visit_with_mut(self));

//     //     for arg in node.args_mut() {
//     //         try_break!(arg.visit_with_mut(self));
//     //     }

//     //     ControlFlow::Continue(())
//     // }

//     // fn visit_pattern_mut(
//     //     &mut self,
//     //     node: &'ast mut boa_ast::pattern::Pattern,
//     // ) -> ControlFlow<Self::BreakTy> {
//     //     ControlFlow::Break(())
//     // }

//     // fn visit_function_mut(
//     //     &mut self,
//     //     _node: &'ast mut boa_ast::function::Function,
//     // ) -> ControlFlow<Self::BreakTy> {
//     //     ControlFlow::Break(())
//     // }

//     // fn visit_arrow_function_mut(
//     //     &mut self,
//     //     _node: &'ast mut boa_ast::function::ArrowFunction,
//     // ) -> ControlFlow<Self::BreakTy> {
//     //     ControlFlow::Break(())
//     // }

//     // fn visit_async_function_mut(
//     //     &mut self,
//     //     _node: &'ast mut boa_ast::function::AsyncFunction,
//     // ) -> ControlFlow<Self::BreakTy> {
//     //     ControlFlow::Break(())
//     // }

//     // fn visit_async_arrow_function_mut(
//     //     &mut self,
//     //     _node: &'ast mut boa_ast::function::AsyncArrowFunction,
//     // ) -> ControlFlow<Self::BreakTy> {
//     //     ControlFlow::Break(())
//     // }

//     // fn visit_class_mut(
//     //     &mut self,
//     //     _node: &'ast mut boa_ast::function::Class,
//     // ) -> ControlFlow<Self::BreakTy> {
//     //     ControlFlow::Break(())
//     // }
// }
