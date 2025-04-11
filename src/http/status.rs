pub struct Status {
    pub code_num: u16,
    pub message: &'static str,
}

impl Status {
    pub const OK: Status = Status {
        code_num: 200,
        message: "OK",
    };
    pub const CREATED: Status = Status {
        code_num: 201,
        message: "Created",
    };
    pub const BAD_REQUEST: Status = Status {
        code_num: 400,
        message: "Bad Request",
    };
    pub const NOT_FOUND: Status = Status {
        code_num: 404,
        message: "Not Found",
    };
}