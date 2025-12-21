use lsp_connection::{LspConnectionState, LspRequestId, LspResponse, LspResponseError};

pub fn invalid_request_state(id: LspRequestId, state: LspConnectionState) -> LspResponse {
    let error = match state {
        LspConnectionState::Started => LspResponseError {
            code: lsp_types::error_codes::SERVER_NOT_INITIALIZED,
            message: "Server is not initialized".into(),
            data: None,
        },
        _ => {
            const JSON_RPC_INVALID_REQUEST: i64 = -32600;
            LspResponseError {
                code: JSON_RPC_INVALID_REQUEST,
                message: "Invalid Request".into(),
                data: None,
            }
        }
    };

    LspResponse {
        id,
        result: None,
        error: Some(error),
    }
}

pub fn invalid_params(id: LspRequestId) -> LspResponse {
    const JSON_RPC_INVALID_PARAMS: i64 = -32602;

    LspResponse {
        id,
        result: None,
        error: Some(LspResponseError {
            code: JSON_RPC_INVALID_PARAMS,
            message: "Invalid Request".into(),
            data: None,
        }),
    }
}
