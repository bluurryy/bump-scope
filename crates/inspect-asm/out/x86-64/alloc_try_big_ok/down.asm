inspect_asm::alloc_try_big_ok::down:
	push rbp
	mov rbp, rsp
	push r15
	push r14
	push r13
	push r12
	push rbx
	and rsp, -512
	sub rsp, 2048
	mov r14, rsi
	mov r15, qword ptr [rsi]
	mov rax, qword ptr [r15]
	xor r13d, r13d
	mov qword ptr [rsp + 504], rax
	sub rax, 1024
	cmovae r13, rax
	mov qword ptr [rsp + 496], rdi
	and r13, -512
	cmp r13, qword ptr [r15 + 8]
	jb .LBB0_5
	mov qword ptr [r15], r13
.LBB0_0:
	mov rax, qword ptr [r14]
	mov rbx, qword ptr [rax]
	lea r12, [rsp + 512]
	mov rdi, r12
	call rdx
	mov edx, 1024
	mov rdi, r13
	mov rsi, r12
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rax, qword ptr [r14]
	mov rcx, qword ptr [rax]
	test byte ptr [r13], 1
	je .LBB0_2
	mov r12d, dword ptr [r13 + 4]
	cmp rbx, rcx
	jne .LBB0_1
	mov rdi, r15
	mov rsi, qword ptr [rsp + 504]
	call qword ptr [rip + bump_scope::bump_scope_guard::Checkpoint::reset_within_chunk@GOTPCREL]
	mov qword ptr [r14], r15
.LBB0_1:
	mov rax, qword ptr [rsp + 496]
	mov dword ptr [rax + 4], r12d
	mov ecx, 1
	jmp .LBB0_4
.LBB0_2:
	add r13, 512
	cmp rbx, rcx
	jne .LBB0_3
	mov qword ptr [rax], r13
.LBB0_3:
	mov rax, qword ptr [rsp + 496]
	mov qword ptr [rax + 8], r13
	xor ecx, ecx
.LBB0_4:
	mov dword ptr [rax], ecx
	lea rsp, [rbp - 40]
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_5:
	mov rdi, r14
	mov rbx, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_sized_in_another_chunk
	mov rdx, rbx
	mov r13, rax
	jmp .LBB0_0
