mod canonical;
mod cursor;

pub use canonical::CanonicalError;
pub use cursor::CursorError;

// use std::borrow::Cow;

// use super::aiserver::v1::ErrorDetails;
// use crate::{
//     app::constant::UNKNOWN,
//     common::model::{ApiStatus, ErrorResponse as CommonErrorResponse},
//     core::{error::cursor::CursorError, model::anthropic},
// };
// use ::base64::{Engine as _, engine::general_purpose::STANDARD_NO_PAD};
// use ::prost::Message as _;
// use ::http::StatusCode;
// use ::serde::{Deserialize, Serialize};

// #[derive(Deserialize)]
// pub struct ChatError {
//     error: ErrorBody,
// }

// #[derive(Deserialize)]
// pub struct ErrorBody {
//     code: String,
//     // message: String, // always: Error
//     details: Vec<ErrorDetail>,
// }

// #[derive(Deserialize)]
// pub struct ErrorDetail {
//     // r#type: String, // always: aiserver.v1.ErrorDetails
//     // debug: ErrorDebug,
//     value: String,
// }

// #[derive(Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct ErrorDebug {
//     error: String,
//     details: ErrorDetails,
//     is_expected: Option<bool>,
// }

// #[derive(Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct ErrorDetails {
//     title: String,
//     detail: String,
//     is_retryable: Option<bool>,
// }

// impl ChatError {
//     #[inline]
//     fn error_details(&self) -> Option<ErrorDetails> {
//         self.error.details.first().and_then(|detail| {
//             STANDARD_NO_PAD
//                 .decode(&detail.value)
//                 .ok()
//                 .map(bytes::Bytes::from)
//                 .and_then(|buf| ErrorDetails::decode(buf).ok())
//         })
//     }

//     #[inline]
//     pub fn into_error_response(self) -> (&'static str, ErrorResponse) {
//         let error_details = match self.error_details() {
//             Some(v) => v,
//             None => {
//                 return (
//                     UNKNOWN,
//                     ErrorResponse {
//                         status: 500,
//                         code: Cow::Borrowed(UNKNOWN),
//                         error: None,
//                     },
//                 );
//             }
//         };

//         let status = error_details.status_code();
//         let error_type = error_details.error_type();

//         (
//             error_type,
//             ErrorResponse {
//                 status,
//                 code: Cow::Owned(self.error.code),
//                 error: error_details.details.map(|custom_details| Error {
//                     message: custom_details.title,
//                     details: custom_details.detail,
//                 }),
//             },
//         )
//     }

//     #[inline]
//     pub fn into_custom_error_message(self) -> (&'static str, Option<CustomErrorMessage>, String) {
//         let error_details = match self.error_details() {
//             Some(v) => v,
//             None => return (UNKNOWN, None, self.error.code),
//         };
//         let status = error_details.status_code();
//         let error_type = error_details.error_type();
//         let code = self.error.code;

//         match error_details.details {
//             Some(details) => (
//                 error_type,
//                 Some(CustomErrorMessage {
//                     code: status,
//                     title: details.title,
//                     detail: details.detail,
//                 }),
//                 code,
//             ),
//             None => (UNKNOWN, None, code),
//         }
//     }
// }

// #[derive(Serialize)]
// pub struct CustomErrorMessage {
//     code: u16,
//     title: String,
//     detail: String,
// }

// impl CustomErrorMessage {
//     #[inline]
//     pub fn status_code(message: &Option<Self>) -> StatusCode {
//         if let Some(message) = message.as_ref()
//             && let Ok(code) = StatusCode::from_u16(message.code)
//         {
//             return code;
//         }
//         StatusCode::INTERNAL_SERVER_ERROR
//     }

//     #[inline]
//     pub fn native_code(message: &Option<Self>, code: &str) -> String {
//         message
//             .as_ref()
//             .map_or_else(|| code.replace("_", " "), |message| message.title.clone())
//     }

//     #[inline]
//     pub fn details(message: &Option<Self>) -> Option<&str> {
//         message.as_ref().map(|message| message.detail.as_str())
//     }

//     #[inline]
//     pub fn into_anthropic(
//         message: Option<Self>,
//         error_type: &'static str,
//         default: String,
//     ) -> anthropic::AnthropicError {
//         let message = message
//             .as_ref()
//             .map(|message| __unwrap!(serde_json::to_string(&message)))
//             .unwrap_or(default);
//         anthropic::ErrorDetail {
//             r#type: error_type,
//             message: Cow::Owned(message),
//         }
//         .into_anthropic()
//     }
// }

// #[derive(Serialize)]
// pub struct ErrorResponse {
//     pub status: u16,
//     pub code: Cow<'static, str>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub error: Option<Error>,
// }

// #[derive(Serialize)]
// pub struct Error {
//     pub message: String,
//     pub details: String,
//     // pub value: String,
// }

// impl ErrorResponse {
//     #[inline]
//     pub fn status_code(&self) -> StatusCode {
//         StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
//     }

//     #[inline]
//     pub fn native_code(&self) -> String {
//         self.error.as_ref().map_or_else(
//             || self.code.replace("_", " "),
//             |error| error.message.clone(),
//         )
//     }

//     #[inline]
//     pub fn details(&self) -> Option<&str> {
//         self.error.as_ref().map(|error| error.details.as_str())
//     }

//     #[inline]
//     pub fn into_generic(self) -> CommonErrorResponse {
//         let error;
//         let message;
//         if let Some(e) = self.error {
//             error = Some(Cow::Owned(e.message));
//             message = Some(Cow::Owned(e.details));
//         } else {
//             error = Some(self.code);
//             message = None;
//         }
//         CommonErrorResponse {
//             status: ApiStatus::Error,
//             code: Some(self.status),
//             error,
//             message,
//         }
//     }
// }

pub enum StreamError {
    Upstream(CursorError),
    DataLengthLessThan5,
    EmptyStream,
}

impl ::core::fmt::Display for StreamError {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        match self {
            Self::Upstream(error) => f.write_str(error.code()),
            Self::DataLengthLessThan5 => f.write_str("data length less than 5"),
            Self::EmptyStream => f.write_str("empty stream"),
        }
    }
}
