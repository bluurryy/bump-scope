inspect_asm::alloc_layout::try_bumpalo:
	mov rcx, qword ptr [rdi + 16]
	mov rax, qword ptr [rcx + 32]
	sub rax, rdx
	jb .LBB_4
	mov r8, rsi
	neg r8
	and rax, r8
	cmp rax, qword ptr [rcx]
	jb .LBB_4
	mov qword ptr [rcx + 32], rax
	test rax, rax
	je .LBB_4
	ret
.LBB_4:
	jmp qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]