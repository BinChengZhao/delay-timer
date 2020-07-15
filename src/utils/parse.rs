pub(crate) mod shell_command {
    use anyhow::*;
    use std::fs::{File,OpenOptions};
    use std::process::{Child, Command, Stdio};


    //that code from 'build-your-own-shell-rust'. Thanks you Josh Mcguigan.

    fn parse_and_run(input: &str) -> Result<Child> {
        // must be peekable so we know when we are on the last command
        let mut commands = input.trim().split(" | ").peekable();
        //if str has >> | > ,after spawn return.
        let mut previous_command = None;

        let mut _stdio: Stdio;

        while let Some(mut command) = commands.next() {
            //没有制定标准输出的重定向，默认给记下来

            let mut angle_bracket = if command.contains(">>") {
                Some(">>")
            } else {
                None
            };

            if angle_bracket.is_none() && command.contains(">") {
                angle_bracket = Some(">");
            }


            let file_stdio:File;

            let mut sub_command ;
            if let Some(symbol) = angle_bracket {

                let mut append_content_command = command.trim().split(symbol).peekable();
                let _command_body = append_content_command.next().unwrap();
                let _filename = append_content_command.next().unwrap();

                let mut file_tmp = OpenOptions::new();
                file_tmp.write(true).create(true);

                 if symbol == ">>"{
                    file_tmp.append(true);
                 }
                use std::iter::Iterator;

                 //如果有重定向，则翻转取最后的字符串作为重定向地址

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
                 

                 //I can give a closer.
                 let mut sub_command_inner = input.trim().split_inclusive(symbol).rev();
                 
                //  sub_command = input.trim().split_inclusive(symbol).rev();


                //把最后的元素当做 重定向的文件名字
                let file_name = sub_command_inner.next().ok_or(anyhow!("file name is faker."))?;
                file_stdio =  file_tmp.open(file_name.trim())?;
                // sub_command.map(|element|{element.strip_suffix(symbol)});
                //再正过来
                sub_command = sub_command_inner.rev();
            }

            //FIXME:priority.
            // process_body eq command.......
            // let process_body = sub_command.next().ok_or(anyhow!("have no process body."));

            
            let mut parts = command.trim().split_whitespace();
            let command = parts.next().unwrap();
            let args = parts;

            let stdin = previous_command.map_or(Stdio::inherit(), |output: Child| {
                Stdio::from(output.stdout.unwrap())
            });

            let stdout = if commands.peek().is_some() {
                // there is another command piped behind this one
                // prepare to send output to the next command
                Stdio::piped()
            } else {
                // there are no more commands piped behind this one
                // send output to shell stdout
                Stdio::inherit()
            };

            let output = Command::new(command)
                .args(args)
                .stdin(stdin)
                .stdout(stdout)
                .spawn();

            match output {
                Ok(output) => {
                    previous_command = Some(output);
                }
                Err(e) => {
                    previous_command = None;
                    eprintln!("{}", e);
                }
            };
        }

        //TODO:
        previous_command.ok_or(anyhow!("can't do that."))
    }
}
