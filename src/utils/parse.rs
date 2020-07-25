pub mod shell_command {
    use anyhow::*;
    use std::fs::{File, OpenOptions};
    use std::iter::Iterator;
    use std::mem;
    use std::path::Path;
    use std::process::{Child, Command, Stdio};

    use std::collections::LinkedList;
    //that code from 'build-your-own-shell-rust'. Thanks you Josh Mcguigan.

    pub fn parse_and_run(input: &str) -> Result<LinkedList<Child>> {
        let mut process_linked_list: LinkedList<Child> = LinkedList::new();
        // must be peekable so we know when we are on the last command
        let mut commands = input.trim().split(" | ").peekable();
        //if str has >> | > ,after spawn return.

        let mut _stdio: Stdio;

        while let Some(mut command) = commands.next() {
            //没有制定标准输出的重定向，默认给记下来

            //TODO:priority.
            // process_body eq command.......
            // let process_body = sub_command.next().ok_or(anyhow!("have no process body."));
            //如果有重定向，则翻转取最后的字符串作为重定向地址

            // if has filename ,remove ''>> filename '' in commond
            // And run a process ,end.

            let check_redirect_result = _has_redirect_file(command);
            if check_redirect_result.is_some() {
                command = _remove_angle_bracket_command(command)?;
            }

            let mut parts = command.trim().split_whitespace();
            let command = parts.next().unwrap();
            let args = parts;

            //当前要生成进程的标准输入
            //如果有previous_command，就继承上一个命令的标准输出
            //没有就继承当前的父进程。
            let previous_command = process_linked_list.back_mut();
            let mut stdin = Stdio::inherit();

            if let Some(previous_command_ref) = previous_command {
                let mut t = None;
                mem::swap(&mut previous_command_ref.stdout, &mut t);

                if let Some(child_stdio) = t {
                    stdin = Stdio::from(child_stdio);
                }
            }
            println!("command:{:?}, args:{:?}", command, args);

            let mut output = Command::new(command);
            output.args(args).stdin(stdin);

            let process;
            let mut end_flag = if check_redirect_result.is_some() {
                println!("check_redirect_result : {:?}", check_redirect_result);
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
    //通过 Some判断，有或没有输出得stdio
    //通过 Some内的Reuslt 判断，有没有open成功
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
            println!("redirect:filename{:?}", filename);
            return Some(create_stdio_file(angle_bracket, filename));
        } else {
            None
        }
    }

    //确认有重定向文件后，解析命令 ‘>’  之前的就是命令
    fn _remove_angle_bracket_command(command: &str) -> Result<&str> {
        let mut sub_command_inner = command.trim().split('>');
        sub_command_inner.next().ok_or(anyhow!("can't parse ...."))
    }

    fn create_stdio_file(angle_bracket: &str, filename: &str) -> Result<File> {
        let mut file_tmp = OpenOptions::new();
        file_tmp.write(true).create(true);

        if angle_bracket == ">>" {
            file_tmp.append(true);
        }

        //在需要返回Result的地方，使用Result<T, anyhow::Error>或者等价的anyhow::Result<T>，
        //就可以利用？抛出任何类型实现了std::error::Error的错误
        //anyhow::Error是与std::error::Error兼容的

        //Path::new("foo.txt").as_os_str()
        //TODO:I need record that open file error because filename has a whitespace i don't trim.
        let os_filename = Path::new(filename.trim()).as_os_str();
        println!("stdio_file:{:?}", file_tmp);
        println!("filename:{:?}", filename);

        println!("os_filename:{:?}", os_filename);

        let stdio_file = file_tmp.open(os_filename)?;
        Ok(stdio_file)
    }

    //error record
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
