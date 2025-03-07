# shellcheck shell=bash

_tldr() {
    local cur="${COMP_WORDS[COMP_CWORD]}"
    local prev="${COMP_WORDS[COMP_CWORD-1]}"

    local opts="-u -l -a -i -r -p -L -o -c -R -q -v -h \
    --update --list --list-all --list-platforms --list-languages \
    --info --render --clean-cache --gen-config --config-path --platform \
    --language --short-options --long-options --offline --compact \
    --no-compact --raw --no-raw --quiet --color --config --version --help"

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
            mapfile -t COMPREPLY < <(compgen -W "$(tldr --offline --list-platforms 2> /dev/null)" -- "$cur");;
        -L|--language)
            mapfile -t COMPREPLY < <(compgen -W "$(tldr --offline --list-languages 2> /dev/null)" -- "$cur");;
        *)
            mapfile -t COMPREPLY < <(compgen -W "$(tldr --offline --list-all 2> /dev/null)" -- "$cur");;
    esac
}

complete -o bashdefault -F _tldr tldr
