inspect_asm::alloc_try_big_ok::bumpalo:
	push rbp
	mov rbp, rsp
	push r15
	push r14
	push r13
	push r12
	push rbx
	and rsp, -512
	sub rsp, 2048
	mov rbx, rdi
	mov r15, qword ptr [rsi + 16]
	mov r13, qword ptr [r15 + 32]
	cmp r13, 1024
	jb .LBB0_5
	lea r14, [r13 - 1024]
	and r14, -512
	cmp r14, qword ptr [r15]
	jb .LBB0_5
	mov qword ptr [r15 + 32], r14
	test r14, r14
	je .LBB0_5
.LBB0_0:
	mov qword ptr [rsp + 504], rsi
	lea r12, [rsp + 512]
	mov rdi, r12
	call rdx
	mov edx, 1024
	mov rdi, r14
	mov rsi, r12
	call qword ptr [rip + memcpy@GOTPCREL]
	test byte ptr [r14], 1
	je .LBB0_3
	mov rax, qword ptr [rsp + 504]
	mov rax, qword ptr [rax + 16]
	cmp qword ptr [rax + 32], r14
	jne .LBB0_2
	cmp rax, r15
	je .LBB0_1
	mov r13, qword ptr [rax]
.LBB0_1:
	mov qword ptr [rax + 32], r13
.LBB0_2:
	mov eax, dword ptr [r14 + 4]
	mov dword ptr [rbx + 4], eax
	mov eax, 1
	jmp .LBB0_4
.LBB0_3:
	add r14, 512
	mov qword ptr [rbx + 8], r14
	xor eax, eax
.LBB0_4:
	mov dword ptr [rbx], eax
	mov rax, rbx
	lea rsp, [rbp - 40]
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_5:
	mov r12, rsi
	mov esi, 512
	mov r14, rdx
	mov edx, 1024
	mov rdi, r12
	call qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
	mov rsi, r12
	mov rdx, r14
	mov r14, rax
	test rax, rax
	jne .LBB0_0
	call qword ptr [rip + bumpalo::oom@GOTPCREL]
