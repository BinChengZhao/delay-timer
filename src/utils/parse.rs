pub mod shell_command {
    use anyhow::*;
    use std::fs::{File, OpenOptions};
    use std::iter::Iterator;
    use std::mem;
    use std::path::Path;
    use std::process::{Child, Command, Stdio};

    use std::collections::LinkedList;
    //that code base on 'build-your-own-shell-rust'. Thanks you Josh Mcguigan.

    ///Generate a list of processes from a string of shell commands.
    // TODO: There are a lot of system calls that happen when this method is called,
    // the speed of execution depends on the parsing of the command and the speed of the process fork,
    // after which it should be split into unblock().
    pub fn parse_and_run(input: &str) -> Result<LinkedList<Child>> {
        // TODO:If there is a parsing error during function execution,
        // should kill all processes that were open before.

        // Check to see if process_linked_list is also automatically dropped out of scope 
        // by ERROR's early return and an internal kill method is executed.

        let mut process_linked_list: LinkedList<Child> = LinkedList::new();
        // must be peekable so we know when we are on the last command
        let mut commands = input.trim().split(" | ").peekable();
        //if str has >> | > ,after spawn return.

        let mut _stdio: Stdio;

        while let Some(mut command) = commands.next() {
            let check_redirect_result = _has_redirect_file(command);
            if check_redirect_result.is_some() {
                command = _remove_angle_bracket_command(command)?;
            }

            let mut parts = command.trim().split_whitespace();
            let command = parts.next().unwrap();
            let args = parts;

            //Standard input to the current process.
            //If previous_command is present, it inherits the standard output of the previous command.
            //If not, it inherits the current parent process.
            let previous_command = process_linked_list.back_mut();
            let mut stdin = Stdio::inherit();

            if let Some(previous_command_ref) = previous_command {
                let mut t = None;
                mem::swap(&mut previous_command_ref.stdout, &mut t);

                if let Some(child_stdio) = t {
                    stdin = Stdio::from(child_stdio);
                }
            }

            let mut output = Command::new(command);
            output.args(args).stdin(stdin);

            let process;
            let end_flag = if check_redirect_result.is_some() {
                let stdout = check_redirect_result.unwrap()?;
                process = output.stdout(stdout).spawn()?;
                true
            } else {
                let stdout;
                if commands.peek().is_some() {
                    // there is another command piped behind this one
                    // prepare to send output to the next command
                    stdout = Stdio::piped();
                } else {
                    // there are no more commands piped behind this one
                    // send output to shell stdout
                    // TODO:It shouldn't be the standard output of the parent process in the context,
                    // there should be a default file to record it.
                    stdout = Stdio::inherit();
                };
                process = output.stdout(stdout).spawn()?;
                false
            };

            process_linked_list.push_back(process);

            if end_flag {
                break;
            }
        }
        Ok(process_linked_list)
    }

    // I should give Option<Result<File>>
    //By Option(Some(Result<T>)), determine if there is an output stdio..
    //By Result<T>(OK(t)), determine if there is success open file.
    fn _has_redirect_file(command: &str) -> Option<Result<File>> {
        let angle_bracket;

        if command.contains(">>") {
            angle_bracket = ">>";
        } else if command.contains(">") {
            angle_bracket = ">";
        } else {
            return None;
        }

        let mut sub_command_inner = command.trim().split_inclusive(angle_bracket).rev();
        if let Some(filename) = sub_command_inner.next() {
            Some(create_stdio_file(angle_bracket, filename))
        } else {
            None
        }
    }

    //After confirming that there is a redirect file, parse the command before the command '>'.
    fn _remove_angle_bracket_command(command: &str) -> Result<&str> {
        let mut sub_command_inner = command.trim().split('>');
        sub_command_inner
            .next()
            .ok_or_else(|| anyhow!("can't parse ...."))
    }

    fn create_stdio_file(angle_bracket: &str, filename: &str) -> Result<File> {
        let mut file_tmp = OpenOptions::new();
        file_tmp.write(true).create(true);

        if angle_bracket == ">>" {
            file_tmp.append(true);
        }

        // Where you need to return a Result, use Result<T, anyhow::Error> or the equivalent anyhow::Result<T>.
        //You can use it? Throws any type of error that implements std::error::Error.
        //anyhow::Error is compatible with std::error::Error.

        //TODO:I need record that open file error because filename has a whitespace i don't trim.
        let os_filename = Path::new(filename.trim()).as_os_str();

        let stdio_file = file_tmp.open(os_filename)?;
        Ok(stdio_file)
    }

    //error record.
    //sub_command = input.trim().split(">>").rev();
    //因为 ">>" 作为一个Patten，内部关联类型Searcher会生成一个 StrSearcher<'a, 'b>
    //StrSearcher<'a, 'b>上面没有标签trait DoubleEndedSearcher
    //所以不满足
    //  impl<'a, P> DoubleEndedIterator for Split<'a, P>
    //  where
    //     P: Pattern<'a>,
    //     <P as Pattern<'a>>::Searcher: DoubleEndedSearcher<'a>,
    //所以不能使用DoubleEndedIterator的，rev方法
    //ps: 'x' is char "x" is str.
}
