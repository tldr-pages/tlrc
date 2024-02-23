# shellcheck shell=bash

_tldr() {
    local cur="${COMP_WORDS[COMP_CWORD]}"
    local prev="${COMP_WORDS[COMP_CWORD-1]}"

    local opts="-u -l -a -i -r -p -L -o -c -R -q -v -h \
    --update --list --list-all --list-platforms --list-languages \
    --info --render --clean-cache --gen-config --config-path --platform \
    --language --offline --compact --no-compact --raw --no-raw --quiet \
    --color --config --version --help"

    if [[ $cur == -* ]]; then
        mapfile -t COMPREPLY < <(compgen -W "$opts" -- "$cur")
        return 0
    fi

    case $prev in
        -r|--render|--config)
            mapfile -t COMPREPLY < <(compgen -f -- "$cur");;
        --color)
            mapfile -t COMPREPLY < <(compgen -W "auto always never" -- "$cur");;
        -p|--platform)
            local platforms
            platforms=$(tldr --offline --list-platforms 2> /dev/null)
            mapfile -t COMPREPLY < <(compgen -W "$platforms" -- "$cur");;
        -L|--language)
            local languages
            languages=$(tldr --offline --list-languages 2> /dev/null)
            mapfile -t COMPREPLY < <(compgen -W "$languages" -- "$cur");;
        *)
            local all
            all=$(tldr --offline --list-all 2> /dev/null)
            mapfile -t COMPREPLY < <(compgen -W "$all" -- "$cur");;
    esac
}

complete -o bashdefault -F _tldr tldr
