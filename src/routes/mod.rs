mod health_check;
mod subscriptions;
mod subscriptions_confirm;

pub(crate) use health_check::health_check;
pub(crate) use subscriptions::subscribe;
pub(crate) use subscriptions_confirm::confirm;

fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;

    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }

    Ok(())
}
