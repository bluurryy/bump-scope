inspect_asm::alloc_iter_u32::down_a:
	push rbp
	push r15
	push r14
	push r13
	push r12
	push rbx
	sub rsp, 40
	mov rbx, rdi
	test rdx, rdx
	je .LBB0_5
	mov rax, rdx
	shr rax, 61
	jne .LBB0_11
	mov r15, rsi
	lea r12, [4*rdx]
	mov rcx, qword ptr [rbx]
	mov rax, qword ptr [rcx]
	mov rsi, rax
	sub rsi, qword ptr [rcx + 8]
	cmp r12, rsi
	ja .LBB0_10
	sub rax, r12
	mov qword ptr [rcx], rax
	je .LBB0_10
.LBB0_0:
	mov qword ptr [rsp + 8], rax
	mov qword ptr [rsp + 16], 0
	mov qword ptr [rsp + 24], rdx
	mov qword ptr [rsp + 32], rbx
	xor r13d, r13d
	lea rbx, [rsp + 8]
	xor r14d, r14d
	jmp .LBB0_2
.LBB0_1:
	mov dword ptr [rax + 4*r14], ebp
	inc r14
	mov qword ptr [rsp + 16], r14
	add r13, 4
	cmp r12, r13
	je .LBB0_3
.LBB0_2:
	mov ebp, dword ptr [r15 + r13]
	cmp qword ptr [rsp + 24], r14
	jne .LBB0_1
	mov rdi, rbx
	call bump_scope::bump_vec::BumpVec<T,A,_,_,_>::generic_grow_cold
	mov rax, qword ptr [rsp + 8]
	mov r14, qword ptr [rsp + 16]
	jmp .LBB0_1
.LBB0_3:
	mov rsi, qword ptr [rsp + 8]
	mov rbx, qword ptr [rsp + 32]
	mov rax, qword ptr [rbx]
	cmp rsi, qword ptr [rax]
	je .LBB0_6
.LBB0_4:
	mov rax, rsi
	jmp .LBB0_9
.LBB0_5:
	mov qword ptr [rsp + 24], rdx
	mov esi, 4
	xor r14d, r14d
	mov rax, qword ptr [rbx]
	cmp rsi, qword ptr [rax]
	jne .LBB0_4
.LBB0_6:
	lea rdx, [4*r14]
	mov rax, qword ptr [rsp + 24]
	lea rax, [rsi + 4*rax]
	xor edi, edi
	sub rax, rdx
	cmovae rdi, rax
	and rdi, -4
	lea rax, [rdx + rsi]
	mov r15, rdi
	cmp rax, rdi
	jbe .LBB0_7
	call qword ptr [rip + memmove@GOTPCREL]
	jmp .LBB0_8
.LBB0_7:
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB0_8:
	mov rcx, qword ptr [rbx]
	mov rax, r15
	mov qword ptr [rcx], r15
	mov qword ptr [rsp + 24], r14
.LBB0_9:
	mov rdx, r14
	add rsp, 40
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_10:
	mov rdi, rbx
	mov rsi, rdx
	mov r14, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_slice_in_another_chunk
	mov rdx, r14
	jmp .LBB0_0
.LBB0_11:
	call qword ptr [rip + bump_scope::private::capacity_overflow@GOTPCREL]
