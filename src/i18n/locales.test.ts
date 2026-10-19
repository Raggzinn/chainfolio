import en from "./locales/en.json";
import ptBR from "./locales/pt-BR.json";

function keys(obj: object, prefix = ""): string[] {
  return Object.entries(obj).flatMap(([k, v]) =>
    typeof v === "object" && v !== null ? keys(v, `${prefix}${k}.`) : [`${prefix}${k}`],
  );
}

describe("locales", () => {
  it("pt-BR has exactly the same keys as en", () => {
    expect(keys(ptBR).sort()).toEqual(keys(en).sort());
  });
});
