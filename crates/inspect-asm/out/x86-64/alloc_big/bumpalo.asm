inspect_asm::alloc_big::bumpalo:
	push rbx
	mov rax, rdi
	mov rcx, qword ptr [rdi + 16]
	mov rdi, qword ptr [rcx + 32]
	cmp rdi, 512
	jb .LBB_3
	add rdi, -512
	and rdi, -512
	cmp rdi, qword ptr [rcx]
	jb .LBB_3
	mov qword ptr [rcx + 32], rdi
	test rdi, rdi
	je .LBB_3
.LBB_5:
	mov edx, 512
	pop rbx
	jmp qword ptr [rip + memcpy@GOTPCREL]
.LBB_3:
	mov rbx, rsi
	mov esi, 512
	mov edx, 512
	mov rdi, rax
	call qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
	mov rsi, rbx
	mov rdi, rax
	test rax, rax
	jne .LBB_5
	call qword ptr [rip + bumpalo::oom@GOTPCREL]