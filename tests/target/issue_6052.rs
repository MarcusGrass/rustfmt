// Passing the max-line boundary seems to be, combined with the other stuff, causing the panic
pub enum Dummy<
    SomeVeryLongStructDeclarationAsItemMakingTheLineOverflowTheRightHandSideAssignmentIsImportant = MyDefault,
> {}
