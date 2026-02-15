use super::*;
use syn::{parse_quote, Field};

#[test]
fn test_unknown_ply_attribute_full_path() {
    let field: Field = parse_quote! {
        #[ply(my::unknown::attr = "foo")]
        my_field: i32
    };
    
    let result = parse_ply_attr(&field);
    match result {
        Ok(_) => panic!("Expected error, but got success"),
        Err(err) => {
            assert_eq!(err.to_string(), "unknown ply attribute: my :: unknown :: attr");
        }
    }
}
