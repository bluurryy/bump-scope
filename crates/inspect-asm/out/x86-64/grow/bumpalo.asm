inspect_asm::grow::bumpalo:
	push rbp
	push r15
	push r14
	push r13
	push r12
	push rbx
	push rax
	mov rbx, r9
	cmp rdx, r8
	jb .LBB0_0
	mov rax, qword ptr [rdi + 16]
	cmp qword ptr [rax + 32], rsi
	je .LBB0_3
.LBB0_0:
	mov rax, qword ptr [rdi + 16]
	mov r14, qword ptr [rax + 32]
	sub r14, rbx
	jb .LBB0_4
	mov rdx, r8
	neg rdx
	and r14, rdx
	cmp r14, qword ptr [rax]
	jb .LBB0_4
	mov qword ptr [rax + 32], r14
	test r14, r14
	je .LBB0_4
.LBB0_1:
	mov rdi, r14
	mov rdx, rcx
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB0_2:
	mov rax, r14
	mov rdx, rbx
	add rsp, 8
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_3:
	mov r14, rdx
	mov r12, rdi
	mov qword ptr [rsp], r8
	mov r13, rsi
	mov r15, rbx
	mov rbp, rcx
	sub r15, rcx
	mov rdi, r15
	mov rsi, rdx
	call qword ptr [rip + core::alloc::layout::Layout::is_size_align_valid@GOTPCREL]
	test al, al
	je .LBB0_5
	mov rdi, r12
	mov rax, qword ptr [r12 + 16]
	mov rdx, qword ptr [rax + 32]
	cmp r15, rdx
	mov rcx, rbp
	mov rsi, r13
	mov r8, qword ptr [rsp]
	ja .LBB0_0
	sub rdx, r15
	neg r14
	and r14, rdx
	cmp r14, qword ptr [rax]
	jb .LBB0_0
	mov qword ptr [rax + 32], r14
	test r14, r14
	je .LBB0_0
	mov rdi, r14
	mov rdx, rcx
	call qword ptr [rip + memmove@GOTPCREL]
	jmp .LBB0_2
.LBB0_4:
	mov r14, rsi
	mov rsi, r8
	mov rdx, rbx
	mov r15, rcx
	call qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
	mov rsi, r14
	mov rcx, r15
	mov r14, rax
	test rax, rax
	jne .LBB0_1
.LBB0_5:
	xor r14d, r14d
	jmp .LBB0_2
