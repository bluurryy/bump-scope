inspect_asm::alloc_str::bumpalo:
	push r14
	push rbx
	push rax
	mov rax, qword ptr [rdi + 16]
	mov rbx, qword ptr [rax + 32]
	sub rbx, rdx
	jb .LBB_3
	cmp rbx, qword ptr [rax]
	jb .LBB_3
	mov qword ptr [rax + 32], rbx
	test rbx, rbx
	je .LBB_3
.LBB_4:
	mov rdi, rbx
	mov r14, rdx
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rax, rbx
	mov rdx, r14
	add rsp, 8
	pop rbx
	pop r14
	ret
.LBB_3:
	mov r14, rsi
	mov esi, 1
	mov rbx, rdx
	call qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
	mov rsi, r14
	mov rdx, rbx
	mov rbx, rax
	test rax, rax
	jne .LBB_4
	call qword ptr [rip + bumpalo::oom@GOTPCREL]