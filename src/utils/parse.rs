//! parse
//! It is a module that parses and executes commands.

/// Collection of functions related to shell commands and processes.
pub mod shell_command {
    use anyhow::*;

    use async_trait::async_trait;

    use smol::process::{Child as SmolChild, Command as SmolCommand};

    use std::collections::LinkedList;
    use std::convert::AsRef;
    use std::ffi::OsStr;
    use std::fs::{File, OpenOptions};
    use std::io::Result as IoResult;
    use std::iter::Iterator;
    // use std::iter::Iterator;
    use std::mem;
    use std::ops::{Deref, DerefMut};
    use std::path::Path;
    use std::process::{Child as StdChild, Command, ExitStatus, Output, Stdio};

    /// The linkedlist of ChildGuard.
    pub type ChildGuardList = LinkedList<ChildGuard>;
    /// The linkedlist of ChildGuard.
    pub type ChildGuardListX<T> = LinkedList<ChildGuardX<T>>;

    macro_rules! impl_command_unify{
        ($($command:ty => $child:ty),+) => {
            $(impl CommandUnify<$child> for $command {
                fn new<S: AsRef<OsStr>>(program: S) -> Self {
                    Self::new(program.as_ref())
                }
                fn args<I, S>(&mut self, args: I) -> &mut Self
                where
                    I: IntoIterator<Item = S>,
                    S: AsRef<OsStr>,
                {
                    self.args(args);
                    self
                }

                fn stdin<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Self {
                    self.stdin(cfg);
                    self
                }

                fn stdout<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Self {
                    self.stdout(cfg);
                    self
                }

                fn spawn(&mut self) -> IoResult<$child> {
                    self.spawn()
                }
            })+
        }
    }

    /// Abstraction of command methods in multiple libraries.
    pub trait CommandUnify<Child: ChildUnify>: Sized {
        /// Constructs a new Command for launching the program at path program.
        fn new<S: AsRef<OsStr>>(program: S) -> Self {
            Self::new(program.as_ref())
        }

        /// Adds multiple arguments to pass to the program.
        fn args<I, S>(&mut self, args: I) -> &mut Self
        where
            I: IntoIterator<Item = S>,
            S: AsRef<OsStr>;

        /// Configuration for the child process's standard input (stdin) handle.
        fn stdin<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Self;

        /// Configuration for the child process's standard output (stdout) handle.
        fn stdout<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Self;

        /// Executes the command as a child process, returning a handle to it.
        fn spawn(&mut self) -> IoResult<Child>;
    }

    impl_command_unify!(Command => StdChild,SmolCommand => SmolChild);
    cfg_tokio_support!(
        use tokio::process::Command as TokioCommand;
        use tokio::process::Child as TokioChild;
        use std::convert::TryInto;
        impl_command_unify!(TokioCommand => TokioChild);
    );

    #[async_trait]
    /// Trait abstraction of multiple library process handles.
    pub trait ChildUnify {
        /// Executes the command as a child process, waiting for it to finish and collecting all of its output.
        async fn wait_with_output(self) -> IoResult<Output>;
        /// Convert stdout to stdio.
        async fn stdout_to_stdio(&mut self) -> Option<Stdio>;
    }

    #[async_trait]
    impl ChildUnify for StdChild {
        async fn wait_with_output(self) -> IoResult<Output> {
            self.wait_with_output()
        }
        async fn stdout_to_stdio(&mut self) -> Option<Stdio> {
            self.stdout.take().map(Stdio::from)
        }
    }

    #[async_trait]
    impl ChildUnify for SmolChild {
        async fn wait_with_output(self) -> IoResult<Output> {
            self.output().await
        }

        async fn stdout_to_stdio(&mut self) -> Option<Stdio> {
            if let Some(stdout) = self.stdout.take() {
                return stdout.into_stdio().await.ok();
            }
            None
        }
    }

    cfg_tokio_support!(
        #[async_trait]
        impl ChildUnify for TokioChild {
            async fn wait_with_output(self) -> IoResult<Output> {
                self.wait_with_output().await
            }

            async fn stdout_to_stdio(&mut self) -> Option<Stdio> {
                self.stdout.take().map(|s| s.try_into().ok()).flatten()
            }
        }
    );
    #[derive(Debug)]
    /// Guarding of process handles.
    pub struct ChildGuard {
        //TODO: 包装tokio/smol 的 Child
        // 从第一个spawn的进程开始wait_output（因为第一个进程没有 stdin, 所以默认被drop也没事）

        //`wait_with_output` The stdin handle to the child process, if any, will be closed before waiting. This helps avoid deadlock: it ensures that the child does not block waiting for input from the parent, while the parent waits for the child to exit.。
        pub(crate) child: StdChild,
    }
    #[derive(Debug)]
    /// Guarding of process handles.
    pub struct ChildGuardX<Child> {
        pub(crate) child: Child,
    }
    impl<Child: ChildUnify> ChildGuardX<Child> {
        pub(crate) fn new(child: Child) -> Self {
            Self { child }
        }

        pub(crate) async fn wait_with_output(self) -> IoResult<Output> {
            self.child.wait_with_output().await
        }
    }

    impl ChildGuard {
        pub(crate) fn new(child: StdChild) -> Self {
            Self { child }
        }
    }

    impl Drop for ChildGuard {
        fn drop(&mut self) {
            self.child.kill().unwrap_or_else(|e| println!("{}", e));
        }
    }

    impl Deref for ChildGuard {
        type Target = StdChild;
        fn deref(&self) -> &Self::Target {
            &self.child
        }
    }

    impl DerefMut for ChildGuard {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.child
        }
    }

    impl<Child: ChildUnify> Deref for ChildGuardX<Child> {
        type Target = Child;
        fn deref(&self) -> &Self::Target {
            &self.child
        }
    }

    impl<Child: ChildUnify> DerefMut for ChildGuardX<Child> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.child
        }
    }
    pub(crate) trait RunningMarker {
        fn get_running_marker(&mut self) -> bool;
    }

    impl RunningMarker for LinkedList<ChildGuard> {
        fn get_running_marker(&mut self) -> bool {
            self.iter_mut()
                .map(|c| c.try_wait())
                .collect::<Vec<IoResult<Option<ExitStatus>>>>()
                .into_iter()
                .any(|c| matches!(c, Ok(None)))
        }
    }

    //that code base on 'build-your-own-shell-rust'. Thanks you Josh Mcguigan.

    /// Generate a list of processes from a string of shell commands.
    // There are a lot of system calls that happen when this method is called,
    //  the speed of execution depends on the parsing of the command and the speed of the process fork,
    //  after which it should be split into unblock().
    pub fn parse_and_run(input: &str) -> Result<ChildGuardList> {
        // Check to see if process_linked_list is also automatically dropped out of scope
        // by ERROR's early return and an internal kill method is executed.

        let mut process_linked_list: ChildGuardList = LinkedList::new();
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
                mem::swap(&mut previous_command_ref.child.stdout, &mut t);

                if let Some(child_stdio) = t {
                    stdin = Stdio::from(child_stdio);
                }
            }

            let mut output = Command::new(command);
            output.args(args).stdin(stdin);

            let process: StdChild;
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

            process_linked_list.push_back(ChildGuard::new(process));

            if end_flag {
                break;
            }
        }
        Ok(process_linked_list)
    }

    /// Generate a list of processes from a string of shell commands.
    //  There are a lot of system calls that happen when this method is called,
    //  the speed of execution depends on the parsing of the command and the speed of the process fork,
    //  after which it should be split into unblock().
    pub async fn parse_and_runx<Child: ChildUnify, Command: CommandUnify<Child>>(
        input: &str,
    ) -> Result<ChildGuardListX<Child>> {
        // Check to see if process_linked_list is also automatically dropped out of scope
        // by ERROR's early return and an internal kill method is executed.

        let mut process_linked_list: ChildGuardListX<Child> = LinkedList::new();
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

                mem::swap(
                    &mut previous_command_ref.child.stdout_to_stdio().await,
                    &mut t,
                );

                if let Some(child_stdio) = t {
                    stdin = child_stdio;
                }
            }

            let mut output = Command::new(command);
            output.args(args).stdin(stdin);

            let process: Child;
            let end_flag = if check_redirect_result.is_some() {
                let stdout = check_redirect_result.unwrap()?;
                process = output.stdout(stdout).spawn()?;
                true
            } else {
                let stdout;
                // if commands.peek().is_some() {
                //     // there is another command piped behind this one
                //     // prepare to send output to the next command
                stdout = Stdio::piped();
                // } else {
                //     // there are no more commands piped behind this one
                //     // send output to shell stdout
                //     // TODO:It shouldn't be the standard output of the parent process in the context,
                //     // there should be a default file to record it.
                //     stdout = Stdio::inherit();
                // };
                process = output.stdout(stdout).spawn()?;
                false
            };

            process_linked_list.push_back(ChildGuardX::<Child>::new(process));

            if end_flag {
                break;
            }
        }
        Ok(process_linked_list)
    }

    // I should give Option<Result<File>>
    //By Option(Some(Result<T>)), determine if there is an output stdio..
    //By Result<T>(OK(t)), determine if there is success open file.
    #[cfg(not(SPLIT_INCLUSIVE_COMPATIBLE))]
    fn _has_redirect_file(command: &str) -> Option<Result<File>> {
        let angle_bracket = if command.contains(">>") {
            ">>"
        } else if command.contains('>') {
            ">"
        } else {
            return None;
        };

        if let Some(filename) = command.trim().rsplit(angle_bracket).next() {
            Some(create_stdio_file(angle_bracket, filename))
        } else {
            None
        }
    }

    #[cfg(SPLIT_INCLUSIVE_COMPATIBLE)]
    fn _has_redirect_file(command: &str) -> Option<Result<File>> {
        let angle_bracket = if command.contains(">>") {
            ">>"
        } else if command.contains('>') {
            ">"
        } else {
            return None;
        };

        let mut sub_command_inner = command.trim().split_inclusive(angle_bracket).rev();

        sub_command_inner
            .next()
            .map(|filename| create_stdio_file(angle_bracket, filename))
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

        //TODO:I need record that open file error because filename has a whitespace i don't trim.
        let os_filename = Path::new(filename.trim()).as_os_str();

        let stdio_file = file_tmp.open(os_filename)?;
        Ok(stdio_file)
    }
}
