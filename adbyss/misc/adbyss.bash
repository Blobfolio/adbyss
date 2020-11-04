_basher__adbyss() {
	local i cur prev opts cmd
	COMPREPLY=()
	cur="${COMP_WORDS[COMP_CWORD]}"
	prev="${COMP_WORDS[COMP_CWORD-1]}"
	cmd=""
	opts=()

	for i in ${COMP_WORDS[@]}; do
		case "${i}" in
			adbyss)
				cmd="adbyss"
				;;

			*)
				;;
		esac
	done

	if [ ! -z "${cmd}" ]; then
		opts=()
		
		if [[ ! " ${COMP_LINE} " =~ " -h " ]] && [[ ! " ${COMP_LINE} " =~ " --help " ]]; then
			opts+=("-h")
			opts+=("--help")
		fi
		[[ " ${COMP_LINE} " =~ " --no-backup " ]] || opts+=("--no-backup")
		[[ " ${COMP_LINE} " =~ " --no-preserve " ]] || opts+=("--no-preserve")
		[[ " ${COMP_LINE} " =~ " --no-summarize " ]] || opts+=("--no-summarize")
		[[ " ${COMP_LINE} " =~ " --stdout " ]] || opts+=("--stdout")
		if [[ ! " ${COMP_LINE} " =~ " -V " ]] && [[ ! " ${COMP_LINE} " =~ " --version " ]]; then
			opts+=("-V")
			opts+=("--version")
		fi
		if [[ ! " ${COMP_LINE} " =~ " -y " ]] && [[ ! " ${COMP_LINE} " =~ " --yes " ]]; then
			opts+=("-y")
			opts+=("--yes")
		fi
		if [[ ! " ${COMP_LINE} " =~ " -c " ]] && [[ ! " ${COMP_LINE} " =~ " --config " ]]; then
			opts+=("-c")
			opts+=("--config")
		fi

		opts=" ${opts[@]} "
		if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
			COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
			return 0
		fi

		case "${prev}" in
			-c|--config)
				COMPREPLY=( $( compgen -f "${cur}" ) )
				return 0
				;;
			*)
				COMPREPLY=()
				;;
		esac

		COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
		return 0
	fi
}

complete -F _basher__adbyss -o bashdefault -o default adbyss
