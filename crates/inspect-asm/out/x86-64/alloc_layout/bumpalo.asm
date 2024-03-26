inspect_asm::alloc_layout::bumpalo:
	push rax
	mov rcx, qword ptr [rdi + 16]
	mov rax, qword ptr [rcx + 32]
	sub rax, rdx
	jb .LBB_3
	mov r8, rsi
	neg r8
	and rax, r8
	cmp rax, qword ptr [rcx]
	jb .LBB_3
	mov qword ptr [rcx + 32], rax
	pop rcx
	ret
.LBB_3:
	call qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
	test rax, rax
	je .LBB_5
	pop rcx
	ret
.LBB_5:
	call qword ptr [rip + bumpalo::oom@GOTPCREL]