inspect_asm::alloc_try_u32::bumpalo:
	push r15
	push r14
	push r13
	push r12
	push rbx
	mov r14, rsi
	mov rbx, rdi
	mov r13, qword ptr [rsi + 16]
	mov r12, qword ptr [r13 + 32]
	cmp r12, 8
	jb .LBB0_5
	lea r15, [r12 - 8]
	and r15, -4
	cmp r15, qword ptr [r13]
	jb .LBB0_5
	mov qword ptr [r13 + 32], r15
	test r15, r15
	je .LBB0_5
.LBB0_0:
	call rdx
	mov dword ptr [r15], eax
	lea rcx, [r15 + 4]
	mov dword ptr [r15 + 4], edx
	test al, 1
	je .LBB0_3
	mov rax, qword ptr [r14 + 16]
	cmp qword ptr [rax + 32], r15
	jne .LBB0_2
	cmp rax, r13
	je .LBB0_1
	mov r12, qword ptr [rax]
.LBB0_1:
	mov qword ptr [rax + 32], r12
.LBB0_2:
	mov eax, dword ptr [rcx]
	mov dword ptr [rbx + 4], eax
	mov eax, 1
	jmp .LBB0_4
.LBB0_3:
	mov qword ptr [rbx + 8], rcx
	xor eax, eax
.LBB0_4:
	mov dword ptr [rbx], eax
	mov rax, rbx
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	ret
.LBB0_5:
	mov esi, 4
	mov r15, rdx
	mov edx, 8
	mov rdi, r14
	call qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
	mov rdx, r15
	mov r15, rax
	test rax, rax
	jne .LBB0_0
	call qword ptr [rip + bumpalo::oom@GOTPCREL]
