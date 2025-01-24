use crate::process::command_exec::decode_output;
use crate::process::command_exec::format_powershell_command;
use crate::process::command_exec::invoke_command;

#[test]
fn test_decode_output_with_hello() {
    let output = vec![72, 101, 108, 108, 111];
    let decoded_output = decode_output(&output);
    assert_eq!(decoded_output, "Hello");
}

#[test]
fn test_decode_output_with_special_character() {
    let output = vec![195, 169, 195, 160, 195, 168];
    let decoded_output = decode_output(&output);
    assert_eq!(decoded_output, "éàè");
}

#[test]
fn test_decode_output_with_wrong_character() {
    // the byte 130 is an invalid utf8 charater
    // and should trigger an error while decoding it using "from_utf8"
    // we are testing that this not causing any failure
    // and it is using the fallback method "from_utf8_lossy"
    let output = vec![72, 101, 108, 108, 111, 130];
    let decoded_output = decode_output(&output);
    assert_eq!(decoded_output, "Hello�");
}

#[ignore]
#[test]
fn test_invoke_command_powershell_special_character() {
    let command = "echo Helloé";
    let formatted_cmd = format_powershell_command(command.to_string());
    let args: Vec<&str> = vec!["-c"];

    let invoke_output = match invoke_command("powershell", &formatted_cmd, args.as_slice()) {
        Ok(output) => output,
        Err(e) => panic!("Failed to invoke PowerShell command: {}", e),
    };

    let stdout = decode_output(&invoke_output.stdout);
    assert_eq!(stdout, "Helloé\r\n");
}
