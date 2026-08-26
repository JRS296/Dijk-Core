/// A type name. At this stage a type is just an identifier — nothing has
/// checked that it refers to anything real; that is semantic analysis's
/// job, still to come.
#[derive(Debug, Clone, PartialEq)]
pub struct Type {
    pub name: String,
}
