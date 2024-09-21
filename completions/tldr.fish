complete -c tldr -s r -l render -d "Render the specified markdown file" -r
complete -c tldr -s p -l platform -d "Specify the platform to use (linux, osx, windows, etc.)" -x -a \
    "(tldr --offline --list-platforms 2> /dev/null)"
complete -c tldr -s L -l language -d "Specify the languages to use" -x -a \
    "(tldr --offline --list-languages 2> /dev/null)"
complete -c tldr -l color -d "Specify when to enable color" -x -a "
    auto\t'Display color if standard output is a terminal and NO_COLOR is not set'
    always\t'Always display color'
    never\t'Never display color'
"
complete -c tldr -l config -d "Specify an alternative path to the config file" -r
complete -c tldr -s u -l update -d "Update the cache"
complete -c tldr -s l -l list -d "List all pages in the current platform"
complete -c tldr -s a -l list-all -d "List all pages"
complete -c tldr -s a -l list-platforms -d "List available platforms"
complete -c tldr -s a -l list-languages -d "List installed languages"
complete -c tldr -s i -l info -d "Show cache information (path, age, installed languages and the number of pages)"
complete -c tldr -l clean-cache -d "Clean the cache"
complete -c tldr -l gen-config -d "Print the default config"
complete -c tldr -l config-path -d "Print the default config path and create the config directory"
complete -c tldr -s o -l offline -d "Do not update the cache, even if it is stale"
complete -c tldr -s c -l compact -d "Strip empty lines from output"
complete -c tldr -l no-compact -d "Do not strip empty lines from output (overrides --compact)"
complete -c tldr -s R -l raw -d "Print pages in raw markdown instead of rendering them"
complete -c tldr -l no-raw -d "Render pages instead of printing raw file contents (overrides --raw)"
complete -c tldr -s q -l quiet -d "Suppress status messages and warnings"
complete -c tldr -s v -l version -d "Print version"
complete -c tldr -s h -l help -d "Print help"
complete -c tldr -f -a "(tldr --offline --list-all 2> /dev/null)"
complete -c tldr -l random -d "Render a random page"
