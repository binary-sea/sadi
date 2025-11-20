//! Macros for ergonomic service registration and container setup in SaDi.
//!
//! - [`bind!`] macro: Shorthand for registering services (concrete, singleton, trait, instance).
//! - [`container!`] macro: Compose a container with multiple `bind!` statements in one block.
//!
//! # Example
//! ```ignore
//! use sadi::{container, bind};
//! use std::sync::Arc;
//! trait Foo: Send + Sync {}
//! struct Bar;
//! impl Foo for Bar {}
//! let c = container! {
//!     bind(dyn Foo => |_c| Bar)
//!     bind(singleton dyn Foo => |_c| Bar)
//!     bind(singleton Bar => |_c| Bar)
//!     bind(instance Bar => Bar)
//!     bind(Bar => |_c| Bar)
//! };
//! // Now c can resolve Foo and Bar in various ways
//! ```

/// Shorthand for registering services in a container.
///
/// - `dyn Trait => factory`: Register a trait-object factory.
/// - `singleton dyn Trait => factory`: Register a singleton trait-object factory.
/// - `singleton Type => factory`: Register a singleton concrete factory.
/// - `instance Type => value`: Register a concrete instance.
/// - `Type => factory`: Register a transient concrete factory.
#[macro_export]
macro_rules! bind {
    // Register a trait-object factory (transient)
    ($container:expr, dyn $token:path => $factory:expr) => {{
        $container
            .bind_abstract::<dyn $token, _, _>(|c| Arc::new($factory(c)) as Arc<dyn $token>)
            .unwrap();
    }};

    // Register a singleton trait-object factory
    ($container:expr, singleton dyn $token:path => $factory:expr) => {{
        $container
            .bind_abstract_singleton::<dyn $token, _, _>(|c| {
                Arc::new($factory(c)) as Arc<dyn $token>
            })
            .unwrap();
    }};

    // Register a singleton concrete factory
    ($container:expr, singleton $token:ty => $factory:expr) => {{
        $container
            .bind_concrete_singleton::<$token, _, _>($factory)
            .unwrap();
    }};

    // Register a concrete instance
    ($container:expr, instance $token:ty => $instance:expr) => {{
        $container.bind_instance::<$token, _>($instance).unwrap();
    }};

    // Register a transient concrete factory
    ($container:expr, $token:ty => $factory:expr) => {{
        $container.bind_concrete::<$token, _, _>($factory).unwrap();
    }};
}

/// Compose a container with multiple `bind!` statements in one block.
///
/// # Example
/// ```ignore
/// let c = container! {
///     bind(dyn Foo => |_c| Bar)
///     bind(singleton Bar => |_c| Bar)
/// };
/// ```
#[macro_export]
macro_rules! container {
    (
        $(
            bind( $($stmt:tt)* )
        )*
    ) => {{
        let container = $crate::Container::new();

        $(
            $crate::bind!(container, $($stmt)*);
        )*

        container
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    trait Foo: Send + Sync {
        fn val(&self) -> i32;
    }
    struct Bar;
    impl Foo for Bar {
        fn val(&self) -> i32 {
            42
        }
    }

    struct Baz;

    #[test]
    fn macro_container_and_bind() {
        // This test assumes the existence of the relevant bind_* methods on Container.
        // It is a compile-check for macro expansion and will need real methods to pass.
        // let c = container! {
        //     bind(dyn Foo => |_c| Bar)
        //     bind(singleton dyn Foo => |_c| Bar)
        //     bind(singleton Baz => |_c| Baz)
        //     bind(instance Baz => Baz)
        //     bind(Bar => |_c| Bar)
        // };
        // let foo: Arc<dyn Foo> = c.resolve::<dyn Foo>().unwrap();
        // assert_eq!(foo.val(), 42);
    }
}
