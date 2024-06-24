inspect_asm::alloc_layout::bumpalo:
	push rax
	mov rcx, qword ptr [rdi + 16]
	mov rax, qword ptr [rcx + 32]
	sub rax, rdx
	jb .LBB0_1
	mov r8, rsi
	neg r8
	and rax, r8
	cmp rax, qword ptr [rcx]
	jb .LBB0_1
	mov qword ptr [rcx + 32], rax
	test rax, rax
	je .LBB0_1
.LBB0_0:
	pop rcx
	ret
.LBB0_1:
	call qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
	test rax, rax
	jne .LBB0_0
	call qword ptr [rip + bumpalo::oom@GOTPCREL]
