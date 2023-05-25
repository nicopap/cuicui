/// Interpret something as an identifier
pub trait GetIdentExt {
    fn get_ident(&self) -> Option<&syn::Ident>;
}
impl GetIdentExt for syn::Type {
    fn get_ident(&self) -> Option<&syn::Ident> {
        use syn::{Type::Path, TypePath};
        let Path(TypePath { qself: None, path }) = self else { return None; };
        path.get_ident()
    }
}
impl GetIdentExt for syn::FnArg {
    fn get_ident(&self) -> Option<&syn::Ident> {
        use syn::{FnArg::Typed, Pat::Ident, PatIdent, PatType};
        let Typed(PatType { pat, .. }) = self else { return None; };
        let Ident(PatIdent { ident, .. }) = &**pat else { return None; };
        Some(ident)
    }
}

/// Convert a collection of errors into a syn error
pub trait IntoSynErrorsExt {
    fn into_syn_errors(self) -> Option<syn::Error>;
}
impl<T: IntoIterator<Item = syn::Error>> IntoSynErrorsExt for T {
    fn into_syn_errors(self) -> Option<syn::Error> {
        self.into_iter().reduce(|mut acc, err| {
            acc.combine(err);
            acc
        })
    }
}
