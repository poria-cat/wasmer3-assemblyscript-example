// We need to declare types of the `host` module exposed by our host
export declare function log(s: String): void;
export declare function getString(): String;

// Due to Assemblyscript data-interop limitations, it can be helpful to write an Assemblyscript shim in front of your
// host-exposed functions to abstract away any interop details.
export function testLog(): void {
  log("Hello wasmer!");
}

export function testGetString(): void {
  let s = getString();
  log(s);
}
