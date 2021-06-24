export enum ErrorKind {
    Unknown,
    ServerError,
    ClientError,
}

export interface ApiError {
    code: number,
    message: string,
}

export enum ErrorCode {
    AlreadyExists = 40001,
    UsernameUnavailable = 40002,
    InvalidEmail = 40003,
    InvalidUsername = 40004,
    InvalidPassword = 40005,
    InvalidCredentials = 40006,
    InsufficientAge = 40007,
    InvalidDate = 40008,
    InvalidContent = 40009,
    InvalidName = 40010,
    InvalidTopic = 40011,
    MissingUploadMetadataHeader = 40012,
    MissingAuthorizationHeader = 40013,
    NoSession = 40014,
    InvalidAuthFormat = 40015,
    HeaderParseError = 40016,
    MissingFilename = 40017,
    MissingFiletype = 40018,
    AuthTokenParseError = 40019,
    Base64DecodeError = 40020,
}

export function errorKind(err: ApiError): ErrorKind {
    if(err.code >= 60000) {
        return ErrorKind.Unknown;
    } else if(err.code >= 50000) {
        return ErrorKind.ServerError;
    } else if(err.code >= 40000) {
        return ErrorKind.ClientError;
    } else {
        return ErrorKind.Unknown;
    }
}

export function parseApiError(err: ApiError): ErrorCode | undefined {
    return ErrorCode[ErrorCode[err.code]];
}