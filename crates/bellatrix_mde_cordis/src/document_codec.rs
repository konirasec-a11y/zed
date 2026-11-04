use cordis::event::{ListenerRegistrationError, responder};
use cordis::{Context, Event};
use std::convert::Infallible;

#[derive(Clone, Debug)]
pub struct ModelingConvertDocumentRequested {
    pub bytes: Vec<u8>,
    pub source_format: String,
    pub destination_format: String,
}

pub struct ModelingConvertDocument;
impl Event for ModelingConvertDocument {
    const NAME: &'static str = "bellatrix.mde.convert-document.v1";
    type Args = ModelingConvertDocumentRequested;
    type Output = Result<Vec<u8>, String>;
}

pub(crate) fn register(context: &Context) -> Result<(), ListenerRegistrationError> {
    let _convert = context.on::<ModelingConvertDocument, _>(responder(
        move |_: Context, request: ModelingConvertDocumentRequested| async move {
            let result = tokio::task::spawn_blocking(move || {
                bellatrix_mde::convert_model_document(
                    &request.bytes,
                    &request.source_format,
                    &request.destination_format,
                )
            }).await.map_err(|error| error.to_string()).and_then(|result| result);
            Ok::<_, Infallible>(Some(result))
        },
    ))?;
    Ok(())
}
