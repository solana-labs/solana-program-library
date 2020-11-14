use ouroboros::self_referencing;

// All tests here should compile and run correctly and pass Miri's safety checks.

#[self_referencing]
struct BoxAndRef {
    data: Box<i32>,
    #[borrows(data)]
    dref: &'this i32,
}

#[self_referencing]
struct BoxAndMutRef {
    data: Box<i32>,
    #[borrows(mut data)]
    dref: &'this mut i32,
}

#[self_referencing(chain_hack, no_doc)]
struct ChainedAndUndocumented {
    data: Box<i32>,
    #[borrows(data)]
    ref1: Box<&'this i32>,
    #[borrows(data)]
    ref2: &'this &'this i32,
}

/// This test just makes sure that the macro copes with a ton of template parameters being thrown at
/// it, specifically checking that the templates work fine even when a generated struct doesn't need
/// all of them. (E.G. heads will only contain 'd, A, and B.)
#[self_referencing]
struct TemplateMess<'d, A, B, C>
where
    A: ?Sized,
    B: 'static,
    C: 'static,
{
    external: &'d A,
    data1: Box<B>,
    #[borrows(data1)]
    data2: &'this C,
    data3: Box<B>,
    #[borrows(mut data3)]
    data4: &'this mut C,
}

#[test]
fn box_and_ref() {
    let bar = BoxAndRefBuilder {
        data: Box::new(12),
        dref_builder: |data| data,
    }
    .build();
    assert!(bar.with_dref(|dref| **dref) == 12);
    drop(bar);
}

#[test]
fn try_new() {
    let bar = BoxAndRefTryBuilder {
        data: Box::new(12),
        dref_builder: |data| Result::<_, ()>::Ok(data),
    }
    .try_build()
    .unwrap();
    assert!(bar.with_dref(|dref| **dref) == 12);
    drop(bar);
}

#[test]
fn try_new_err() {
    let result = BoxAndRefTryBuilder {
        data: Box::new(12),
        dref_builder: |_data| Err(56),
    }
    .try_build();
    if let Err(56) = result {
        // okay
    } else {
        panic!("Test failed.");
    }
}

#[test]
fn try_new_recover_heads() {
    let result = BoxAndRefTryBuilder {
        data: Box::new(12),
        dref_builder: |_data| Err(56),
    }
    .try_build_or_recover();
    if let Err((56, heads)) = result {
        assert!(heads.data == Box::new(12));
    } else {
        panic!("Test failed.");
    }
}

#[test]
fn into_heads() {
    let bar = BoxAndRefBuilder {
        data: Box::new(12),
        dref_builder: |data| data,
    }
    .build();
    assert!(bar.into_heads().data == Box::new(12));
}

#[test]
fn box_and_mut_ref() {
    let mut bar = BoxAndMutRefBuilder {
        data: Box::new(12),
        dref_builder: |data| data,
    }
    .build();
    assert!(bar.with_dref(|dref| **dref) == 12);
    bar.with_dref_mut(|dref| **dref = 34);
    assert!(bar.with_dref(|dref| **dref) == 34);
}

#[test]
fn template_mess() {
    let ext_str = "Hello World!".to_owned();
    let mut instance = TemplateMessBuilder {
        external: &ext_str[..],
        data1: Box::new("asdf".to_owned()),
        data2_builder: |data1_contents| data1_contents,
        data3: Box::new("asdf".to_owned()),
        data4_builder: |data3_contents| data3_contents,
    }
    .build();
    instance.with_external(|ext| println!("{}", ext));
    instance.with_data1_contents(|con| println!("{}", con));
    instance.with_data4_mut(|con| **con = "Modified".to_owned());
    instance.with(|fields| {
        assert!(fields.data1_contents == *fields.data2);
        assert!(*fields.data4 == "Modified");
    });
}

#[cfg(not(feature = "miri"))]
mod compile_tests {
    /// Tests that all files in fail_tests fail to compile.
    #[test]
    fn fails_ok() {
        let t = trybuild::TestCases::new();
        t.compile_fail("src/fail_tests/*.rs");
    }
}

#[allow(dead_code)]
mod test_hygiene {
    mod std {}
    mod core {}

    struct Copy;
    struct Send;
    struct Sync;
    struct Sized;

    struct Drop;
    struct Fn;
    struct FnMut;
    struct FnOnce;

    struct Result;
    struct Ok;
    struct Err;
    struct Option;
    struct Some;
    struct None;

    fn drop() {}

    #[ouroboros::self_referencing]
    struct BoxAndRef {
        data: Box<i32>,
        #[borrows(data)]
        dref: &'this i32,
    }
}
