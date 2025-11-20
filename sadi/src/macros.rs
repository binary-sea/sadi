#[macro_export]
macro_rules! bind {
    ($container:expr, dyn $token:path => $factory:expr) => {{
        $container
            .bind_abstract::<dyn $token, _, _>(|c| Arc::new($factory(c)) as Arc<dyn $token>)
            .unwrap();
    }};

    ($container:expr, singleton dyn $token:path => $factory:expr) => {{
        $container
            .bind_abstract_singleton::<dyn $token, _, _>(|c| {
                Arc::new($factory(c)) as Arc<dyn $token>
            })
            .unwrap();
    }};

    ($container:expr, singleton $token:ty => $factory:expr) => {{
        $container
            .bind_concrete_singleton::<$token, _, _>($factory)
            .unwrap();
    }};

    ($container:expr, instance $token:ty => $instance:expr) => {{
        $container
            .bind_instance::<$token, _>($instance)
            .unwrap();
    }};

    ($container:expr, $token:ty => $factory:expr) => {{
        $container
            .bind_concrete::<$token, _, _>($factory)
            .unwrap();
    }};
}

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
