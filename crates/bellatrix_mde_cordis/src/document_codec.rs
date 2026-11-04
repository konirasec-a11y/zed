pub use bellatrix_mde_contracts::events::{
    ModelingConvertDocument, ModelingConvertDocumentRequested,
};
use cordis::event::{ListenerRegistrationError, responder};
use cordis::{Context, Event};
use std::convert::Infallible;

pub(crate) fn register(context: &Context) -> Result<(), ListenerRegistrationError> {
    let _convert = context.on::<ModelingConvertDocument, _>(responder(
        move |_: Context, request: ModelingConvertDocumentRequested| async move {
            let result = tokio::task::spawn_blocking(move || {
                bellatrix_mde::convert_model_document(
                    &request.bytes,
                    &request.source_format,
                    &request.destination_format,
                )
            })
            .await
            .map_err(|error| error.to_string())
            .and_then(|result| result);
            Ok::<_, Infallible>(Some(result))
        },
    ))?;
    Ok(())
}
