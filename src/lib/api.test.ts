import { ApiError, toApiError } from "./api";

describe("toApiError", () => {
  it("keeps the code of errors serialized by Rust", () => {
    const error = toApiError({ code: "duplicate", message: "already tracked" });
    expect(error).toBeInstanceOf(ApiError);
    expect(error.code).toBe("duplicate");
    expect(error.message).toBe("already tracked");
  });

  it("maps unknown codes and plain errors to unknown", () => {
    expect(toApiError({ code: "weird" }).code).toBe("unknown");
    expect(toApiError(new Error("boom"))).toMatchObject({ code: "unknown", message: "boom" });
    expect(toApiError("ipc closed").message).toBe("ipc closed");
  });
});
