use crate::config::{DavMethod, Permission};

pub fn required_permission(method: DavMethod) -> Option<Permission> {
    match method {
        DavMethod::Get | DavMethod::Head | DavMethod::Propfind => Some(Permission::Read),
        DavMethod::Put => Some(Permission::Write),
        DavMethod::Proppatch => Some(Permission::Write),
        DavMethod::Delete => Some(Permission::Delete),
        DavMethod::Mkcol => Some(Permission::Mkdir),
        DavMethod::Move => Some(Permission::Move),
        DavMethod::Copy => Some(Permission::Copy),
        DavMethod::Lock | DavMethod::Unlock => Some(Permission::Write),
        DavMethod::Options => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_dav_methods_to_permissions() {
        assert_eq!(required_permission(DavMethod::Get), Some(Permission::Read));
        assert_eq!(required_permission(DavMethod::Put), Some(Permission::Write));
        assert_eq!(
            required_permission(DavMethod::Delete),
            Some(Permission::Delete)
        );
        assert_eq!(required_permission(DavMethod::Options), None);
    }
}
